use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use runtime_primitives::traits::{AccountIdConversion, Hash, StaticLookup, Zero};
use runtime_primitives::{ModuleId, Permill};
use support::traits::{ChangeMembers, Currency, Get, ReservableCurrency};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// START TYPES
type ProposalCount = u32;
type Shares = u32;
type MemberCount = u32;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

const DAO_ID: ModuleId = ModuleId(*b"py/daofi");

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Application<AccountId, Balance, BlockNumber> {
    // requested recipient of shares
    applicant: AccountId,
    // the applicant's donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,
    // BlockNumber at initial application
    start: BlockNumber,
}

/// Spending Proposal
///
/// separate fields into multiple structs (create Election struct?)
/// `=>` would need to link Election to every Proposal
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, Balance, BlockNumber, Application> {
    /// Sponsor Information
    // the proposal sponsor must be a member
    sponsor: AccountId,
    // the sponsor's bond
    sponsor_bond: Balance,

    /// Application Information
    // the applicant does not need to be a member
    applicant: AccountId,
    // applicant;s donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,

    /// (Election State)
    // BlockNumber at initial proposal
    vote_start: BlockNumber, // TODO: eventually consider adding GraceWindow and VoteWindow; unique to every proposal (~complexity for flexibility)
    grace_end: Option<BlockNumber>, // initiated by proposal passage in `vote`
    // threshold for passage
    threshold: Shares, // TODO: update based on dilution dynamics in `vote` (like in MolochDAO); calculate_threshold based on donation/shares ratio relative to average
    // shares in favor, shares against
    election_state: (Shares, Shares),
    // supporting voters (voted yes)
    ayes: Vec<(AccountId, Shares)>,
    // against voters (voted no)
    nays: Vec<(AccountId, Shares)>, // TODO: if enough no votes, then bond slashed `=>` otherwise returned
}

/// Voter state (for lock-in + 'instant' withdrawals)
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Member<AccountId, Balance> {
    // accountId directly associated with membership
    member_id: AccountId,
    // total shares owned by the member
    all_shares: Shares,
    // shares reserved based on pending proposal support
    reserved_shares: Shares,
    // active voters are bonded
    voter_bond: Option<Balance>, // TODO: use some `if let Some` pattern for checking
}
// END TYPES

pub trait Trait: system::Trait {
    /// The balances type
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// What to do when the members change
    type ChangeMembers: ChangeMembers<Self::AccountId>;
    // TODO: use this

    /// Base proposal bond, true bond depends on pot size and pending proposals (see `fn proposal_bond`)
    type ProposalBond: Get<BalanceOf<Self>>;

    /// Base voter bond, true bond depends on outstanding voters and pending proposals (see `fn vote_bond`)
    type VoteBond: Get<BalanceOf<Self>>;

    /// Period for which applications are valid after initial application
    type ApplicationWindow: Get<Self::BlockNumber>;

    /// Period for which votes are valid after initial proposal
    type VoteWindow: Get<Self::BlockNumber>;

    /// Period after proposal acceptance during which *commitment-free* dissenters may exit
    type GraceWindow: Get<Self::BlockNumber>;

    /// Frequency with which stale proposals and applications are purged
    /// -- purpose: mitigate state bloat
    type PurgeFrequency: Get<Self::BlockNumber>;

    /// Frequency with which passed proposals are executed
    type ExecuteFrequency: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        BlockNumber = <T as system::Trait>::BlockNumber,
        Hash = <T as system::Trait>::Hash,
        Balance = BalanceOf<T>,
    {
        // applicant AccountId, proposal Hash, donation amount (if any), shares requested
        Applied(AccountId, Hash, Option<Balance>, Shares),
        // sponsor AccountId, proposal Hash, shares requested, block number
        Proposed(AccountId, Hash, Shares, BlockNumber),
        // new voter AccountId, BlockNumber at registration
        RegisterVoter(AccountId, BlockNumber),
        // old voter AccountId, BlockNumber at deregistration
        DeRegisterVoter(AccountId, BlockNumber),
        // voter AccountId, block number, proposal Hash, vote bool, yes_count, no_count
        Voted(AccountId, BlockNumber, Hash, bool, MemberCount, MemberCount), // TODO: add BlockNumber
        // new member AccountId, shares issued, balance committed
        SharesIssued(AccountId, Shares, Balance),
        // member ID, shares burned, balance returned
        SharesBurned(AccountId, Shares, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as MiniDAO {
        /// Applications that are awaiting sponsorship
        Applications get(applications): map T::Hash => Application<AccountId, Balance, BlockNumber>;
        /// All applicants
        Applicant get(applicant): Vec<T::AccountId>;
        /// Applications that have grown stale, awaiting purging
        StaleApps get(stale_apps): Vec<T::Hash>;

        /// Proposals that have been made.
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber, Application>>;
        /// Number of active proposals
        TotalProposals get(total_proposals): ProposalCount;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        StaleProposals get(stale_proposals): Vec<T::Hash>;

        // Outstanding shares requested (directly proposed || apply + sponsor)
        SharesRequested get(shares_requested): Shares;
        // Total shares issued
        SharesIssued get(shares_issued): Shares;

        /// Members of the DAO
        Member get(member): Vec<T::AccountId>;
        /// Total member count
        TotalMembers get(total_members): MemberCount;
        /// Tracking membership shares (voting strength)
        MemberInfo get(member_info): map T::AccountId => Member<T::AccountId, BalanceOf<T>>;
        /// Active voting members (requires additional bond)
        Voter get(voter): Vec<T::AccountId>;
        /// Total active voter count (requires additional bond)
        TotalVoters get(voter_count): MemberCount;

        // TODO: re-add proxy functionality; consider alternative APIs
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Bond for proposals, to be returned
        const ProposalBond: BalanceOf<T> = T::ProposalBond::get();

        /// Bond for active voters, to be returned
        const VoteBond: BalanceOf<T> = T::VoteBond::get();

        /// Period in which applications are valid after initial application
        const ApplicationWindow: T::BlockNumber = T::ApplicationWindow::get();

        /// Period after initial proposal during which voting is allowed
        const VoteWindow: T::BlockNumber = T::VoteWindow::get();

        /// Period after proposal passage during which *commitment-free* dissents can exit early
        const GraceWindow: T::BlockNumber = T::BlockNumber::get();

        /// Period between stale proposal purges
        const PurgeFrequency: T::BlockNumber = T::PurgeFrequency::get();

        /// Period between successive batch execution of passed proposals
        const ExecuteFrequency: T::BlockNumber = T::ExecuteFrequency::get();

        fn apply(
            origin,
            requested_shares: Shares,
            donation: Option<BalanceOf<T>>,
        ) -> Result {
            let applicant = ensure_signed(origin)?;
            // can be commended to remove restriction of one application per pending applicant (otherwise, need better spam protection)
            ensure!(!Self::is_applicant(&applicant), "only one application per applicant");

            // reserve donation if it exists
            if let Some(donate) = donation {
                T::Currency::reserve(&applicant, donate)
                    .map_err(|_| "applicant can't afford donation")?;
            }

            let c = Self::total_apps() + 1;
            TotalApps::put(c); // TODO: replace with #3

            let start = <system::Module<T>>::block_number();
            // clone applicant for event emission post app insertion
            let a = applicant.clone();
            let app = Application {
                applicant,
                donation,
                requested_shares,
                start,
            };
            // take hash of application
            let hash = <T as system::Trait>::Hashing::hash_of(&app);
            // insert application
            <Applications<T>>::insert(hash, &app);
            // add applicant to applicant pool
            <Applicant<T>>::mutate(|applicants| applicants.push(a.clone())); // TODO: `append` once 3071
            Ok(())
        }

        fn sponsor(
            origin,
            app_hash: T::Hash,
            support: Shares,
        ) -> Result {
            let sponsor = ensure_signed(origin)?;
            ensure!(Self::is_member(&sponsor), "sponsor must be a member");

            let mut app = Self::applications(&app_hash).ok_or("application must exist")?;

            // TODO: better system for managing pending applications (better purge)
            if app.start + T::ApplicationWindow::get() < <system::Module<T>>::block_number() {
                <StaleApps<T>>::mutate(|s| s.push(app_hash.clone())); // TODO: `append` after 3071 merged
                return Err("The window has passed for this application");
            }

            // make the proposal
            Self::do_propose(&sponsor, app.applicant, support, app.donation, app.shares)?;

            // remove the application, the proposal is live
            <Applications<T>>::remove(app_hash);

            Ok(())
        }

        /// Direct proposals made by members for members (no apply + sponsor)
        ///
        /// This is a special case in which an existing member would like to request more shares
        /// -- the `calculate_threshold` must take into account the support given by this member and offset it
        /// --
        fn propose(
            origin,
            shares_requested: Shares,
            donation: Option<BalanceOf<T>>,
        ) -> Result {
            let proposer = ensure_signed(origin)?;
            ensure!(Self::is_member(&proposer), "direct proposals can only come from members");
            // ensure the member doesn't have a pending application (TODO: document this *dumb* mechanism design, intention is what matters)
            ensure!(!Self::is_applicant(&proposer), "only one application per applicant");

            // reserve donation if it exists
            if let Some(donate) = donation {
                T::Currency::reserve(&proposer, donate)
                    .map_err(|_| "proposer can't afford donation")?;
            }

            // direct proposal
            Self::do_propose(&proposer, &proposer, 0, donation, shares_requested);
            Ok(())
        }

        // abort function
        //
        // -- would require an abort window

            // register as a new voter
        pub fn register(origin) -> Result {
            let new_voter = ensure_signed(origin)?;
            ensure!(Self::is_member(&new_voter), "every voter must be a member");
            ensure!(!Self::is_voter(&new_voter), "must not be an active voter yet");

            let vote_bond = Self::vote_bond(&new_voter);
            T::Currency::reserve(&new_voter, T::VoteBond::get())
                .map_err(|_| "member doesn't have enough free balance for vote bond")?;

            <Voter<T>>::mutate(|v| v.push(new_voter.clone())); // replace with append once
            let start = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::RegisterVoter(new_voter, start));
            Ok(())
        }

        // deregister as a new voter
        pub fn deregister(origin) -> Result {
            let old_voter = ensure_signed(origin)?;
            ensure!(Self::is_voter(&old_voter), "must be an active voter");

            // TODO: measure instances of misbehavior in a stateful way; slash some percent here
            T::Currency::unreserve(&old_voter, T::VoteBond::get());

            <Voter<T>>::get().retain(|v| v != &old_voter);
            let end = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::DeRegisterVoter(old_voter, end));
            Ok(())
        }

        /// Vote
        ///
        /// -- separate from `do_vote` to facilitate proxy functionality soon
        fn vote(origin, proposal_hash: T::Hash, support: Shares, approve: bool) -> Result {
            let voter = ensure_signed(origin)?;
            ensure!(Self::is_voter(&voter), "The member must be an active voter");

            Self::do_vote(voter, proposal_hash, support, approve)
        }

        fn on_finalize(n: T::BlockNumber) {

            // PURGE
            if (n % T::PurgeFrequency::get()).is_zero() {
                Self::purge();
            }

            // SPEND
            let mut budget = Self::pot();
            if (n % T::ExecuteFrequency::get()).is_zero() {
                budget = Self::spend(budget); // more nuanced
            }// TODO: is_zero() requires the Zero trait from runtime_primitives::traits

            Self::deposit_event(RawEvent::BudgetRemaining(budget));
        }
    }
}

// treasury
impl<T: Trait> Module<T> {
    // dao accountId
    pub fn account_id() -> T::AccountId {
        DAO_ID.into_account()
    } // TODO: requires trait AccountIdConversion

    // total funds in DAO
    fn pot() -> BalanceOf<T> {
        T::Currency::free_balance(&Self::account_id())
    }
}

// supporting methods
impl<T: Trait> Module<T> {
    pub fn is_applicant(who: &T::AccountId) -> bool {
        Self::applicant().contains(who)
    }
    pub fn is_member(who: &T::AccountId) -> bool {
        Self::member().contains(who)
    }
    pub fn is_voter(who: &T::AccountId) -> bool {
        Self::voter().contains(who)
    }

    // calculate proposal bond
    pub fn proposal_bond(shares_requested: Shares, donation: Option<BalanceOf<T>>) -> BalanceOf<T> {
        // check total shares requested
        // vs total shares issued vs this share request
        // vs this donation

        // ideal donation conforms to DAO balance/shares issued ratio
        // TODO: make adjustable and able to vote on this
        // let ideal = Permill::from_percent(10) *
        if let Some(val) = donation {
            // check how ratio of this value to shares_requested is relative to
            return Permill::from_percent(10) * val;
        }
        Permill::from_percent(5) * Self::pot()
    }

    // TODO: based on outstanding voters
    fn vote_bond(new_voter: AccountId) -> BalanceOf<T> {
        // TODO: make more flexible
        let temp_to_change = Permill::from_percent(10) * Self::pot();

        // add to member voter bond information
        temp_to_change
    }

    fn calculate_threshold(shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Shares {
        // TODO: should depend on inputs, total outstanding requests, and other factors (like the proposal_bond)

        // temporary hard-coded threshold (> 20% of outstanding shares)
        let total_shares = Self::shares_issued();
        let threshold = total_shares / 5 + 1;
    }

    // depends on a defined target number of members
    // {`calculate_entry_fee`, `calculate_vote_bond`, `calculate_spend_frequency`, `calculate_threshold`}

    // !--private functions--!

    /// Proposals (of the spend variety)
    ///
    /// Called by `propose` and `proxy_propose`
    fn do_propose(
        sponsor: T::AccountId,
        applicant: T::AccountId,
        support: Shares,
        donation: Option<BalanceOf<T>>,
        shares_requested: Shares,
    ) -> Result {
        // bond the proposer
        let sponsor_bond = Self::proposal_bond(shares_requested, donation);
        T::Currency::reserve(&proposer, sponsor_bond).map_err(|_| "Proposer's balance too low")?;

        let threshold = Self::calculate_threshold(shares_requested, donation);

        let member = Self::member_info(&proposer);
        let free_shares = member.all_shares - member.reserved_shares;
        ensure!(
            support < free_shares,
            "not enough free shares to signal support during sponsor"
        );
        <MemberInfo<T>>::mutate(&proposer, |m| m.reserved_shares += support);

        // start the voting period
        let vote_start = <system::Module<T>>::block_number();
        let grace_end = None;

        // ayes and nays vectors
        let mut ayes = Vec::new();
        let mut nays: Vec<(T::AccountId, Shares)> = Vec::new();
        ayes.push((proposer.clone(), support));

        // clone proposer for event emission after proposal insertion
        let s = sponsor.clone();
        let proposal = Proposal {
            sponsor,
            sponsor_bond,
            applicant,
            donation,
            shares_requested,
            vote_start,
            grace_end,
            threshold,
            ayes,
            nays,
        };
        // take hash of proposal
        let hash = <T as system::Trait>::Hashing::hash_of(&proposal);
        // insert proposal
        <Proposals<T>>::insert(hash, &proposal);
        Self::deposit_event(RawEvent::Proposed(s, hash, shares_requested, vote_start));
        Ok(())
    }

    /// Voting
    ///
    /// Called by `vote` and `proxy_vote`
    fn do_vote(
        voter: T::AccountId,
        proposal_hash: T::Hash,
        support: Shares,
        approve: bool,
    ) -> Result {
        // verify proposal existence
        let mut p = Self::proposals(&proposal_hash).ok_or("proposal must exist")?;

        let time = <system::Module<T>>::block_number();

        if p.vote_start + T::VoteWindow::get() < time {
            <Stale<T>>::mutate(|s| s.push(proposal_hash.clone())); // TODO: update with `append` once PR merged
            return Err("The voting period is over for this proposal");
        }

        let position_yes = p.ayes.iter().position(|a| a.0 == &voter);
        let position_no = p.nays.iter().position(|a| a.0 == &voter);

        if approve {
            if position_yes.is_none() {
                p.ayes.push((voter.clone(), support));
                p.election_state.0 += support;
            } else {
                return Err("duplicate vote");
            }
            // executes if the previous vote was no
            if let Some(pos) = position_no {
                // ability to change vote at no cost prevents bribery attacks
                p.nays.swap_remove(pos);
                p.election_state.1 -= pos.1;
                p.election_state.0 += pos.1;
            }
        } else {
            if position_no.is_none() {
                p.nays.push((voter.clone(), support));
                p.election_state.1 += support;
            } else {
                return Err("duplicate vote");
            }
            if let Some(pos) = position_yes {
                p.ayes.swap_remove(pos);
                p.election_state.0 -= pos.1;
                p.election_state.1 += pos.1
            }
        }

        // abstract a vote count method
        // we don't want to recalculate for every vote
        let total_support = p.election_state.0;
        let total_against = p.election_state.1;
        // TODO: p.threshold should be updated if dynamics have changed?

        if total_support > p.threshold {
            <Passed<T>>::mutate(|pass| pass.push(proposal_hash)); // TODO: update with `append` once 3071 merged
        }

        // TODO: add path in which negative feedback rises above some threshold
        // should slash some of the proposal bond in this event

        Self::deposit_event(RawEvent::Voted(
            voter,
            time,
            proposal_hash,
            approve,
            total_support,
            total_against,
        ));
        Ok(())
    }

    /// Execution
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn execute(mut budget_remaining: BalanceOf<T>) -> BalanceOf<T> {
        let mut missed_any = false;
        <Passed<T>>::mutate(|v| {
            v.retain(|&hash| {
                if let Some(p) = Self::proposals(hash) {
                    if p.value <= budget_remaining {
                        budget_remaining -= p.value;
                        <Proposals<T>>::remove(hash);
                        let total = Self::total_proposals() - 1;
                        <TotalProposals>::put(total);

                        //might require checks that I'm neglecting...? case of multiple proposals?

                        // return bond
                        let _ = T::Currency::unreserve(&p.proposer, p.bond);

                        // transfer the funds
                        let _ = T::Currency::transfer(&Self::account_id(), &p.beneficiary, p.value);

                        Self::deposit_event(RawEvent::Paid(hash, p.value, p.beneficiary));
                        false
                    } else {
                        missed_any = true;
                        true
                    }
                } else {
                    false
                }
            });
        });
        budget_remaining
    }

    /// Purging stale proposals
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn purge() {
        // use type state to consume lazy iterator adaptor
        // TODO: ask for review and discuss
        let _ = <Stale<T>>::get().into_iter().for_each(|h| {
            <Proposals<T>>::remove(h);
            let total = Self::total_proposals() - 1;
            <TotalProposals>::put(total);
        });
        <Stale<T>>::kill();
    }
}

// TESTING
#[cfg(test)]
mod tests {
    use super::RawEvent;
    use super::*;
    use crate::tests::*;
    use crate::tests::{Call, Event as OuterEvent, Origin};
    use runtime_primitives::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup, OnFinalize},
    };
    use support::{assert_noop, assert_ok, Hashable};

    #[test]
    fn basic_setup_works() {
        with_externalities(&mut new_test_ext(), || {
            assert_eq!(MiniDAO::pot(), 0);
            assert_eq!(MiniDAO::proposal_count(), 0);
        });
    }

    #[test]
    fn add_existing_member_fails() {
        assert_eq!(1, 1);
    }

    #[test]
    fn non_member_exit_fails() {
        assert_eq!(1, 1);
    }

    #[test]
    fn spend_proposal_takes_min_deposit() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(MiniDAO::propose_spend(Origin::signed(0), 1, 3));
            assert_eq!(Balances::free_balance(&0), 99);
            assert_eq!(Balances::reserved_balance(&0), 1);
        });
    }

    #[test]
    fn register_nonmember() {
        // nonmember cannot register
        assert_eq!(1, 1);
    }

    #[test]
    fn exit_voter() {
        // active voter cannot leave the DAO
        assert_eq!(1, 1);
    }

    // test proxy
}
