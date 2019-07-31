/// TODO
///
/// - rethink all fields of the structs to minimize calls to storage
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
pub struct Proposal<AccountId, Balance> {
    // the proposal sponsor must be a member
    sponsor: AccountId,
    // the sponsor's bond
    sponsor_bond: Balance,
    // the applicant does not need to be a member
    applicant: AccountId,
    // applicant;s donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,
}

/// Election State
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Election<AccountId, BlockNumber> {
    // applicant accountId (to prevent when == &voter)
    applicant: AccountId,
    // BlockNumber at initial proposal
    vote_start: BlockNumber, // TODO: eventually consider adding GraceWindow and VoteWindow; unique to every proposal (~complexity for flexibility)
    // initiated by proposal passage in `vote`
    grace_end: Option<BlockNumber>,
    // threshold for passage
    threshold: Shares, // TODO: update based on dilution dynamics in `vote` (like in MolochDAO); calculate_threshold based on donation/shares ratio relative to average
    // shares in favor, shares against
    state: (Shares, Shares),
    // supporting voters (voted yes)
    ayes: Vec<(AccountId, Shares)>,
    // against voters (voted no)
    nays: Vec<(AccountId, Shares)>, // TODO: if enough no votes, then bond slashed `=>` otherwise returned
}

/// Voter state (for lock-in + 'instant' withdrawals)
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Member<Balance> {
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

    /// Frequency with which stale proposals and applications are swept
    /// -- purpose: mitigate state bloat
    type SweepFrequency: Get<Self::BlockNumber>;

    /// Frequency with which passed proposals are executed
    type IssuanceFrequency: Get<Self::BlockNumber>;
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
        const SweepFrequency: T::BlockNumber = T::SweepFrequency::get();

        /// Period between successive batch execution of passed proposals
        const IssuanceFrequency: T::BlockNumber = T::IssuanceFrequency::get();

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
            // if the sponsor is the applicant, there are required restrictions on voting for self (see `fn propose`)
            ensure(&app.applicant != &sponsor, "direct proposals should be made via `propose`");

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
            T::Currency::reserve(&new_voter, vote_bond)
                .map_err(|_| "member doesn't have enough free balance for vote bond")?;

            <Voter<T>>::mutate(|v| v.push(new_voter.clone())); // `append` once 3071 merged
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

            <Voter<T>>::mutate(|voters| voters.retain(|v| v != &old_voter));
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

        /// Burn shares
        fn burn(origin, to_burn: Shares) -> Result {
            ensure!(to_burn > 0, "can only burn > 0 shares");
            let arsonist = ensure_signed(origin)?;
            let mut member = Self::member_info(&arsonist).ok_or("must be a member to burn shares")?;
            let free_shares = member.all_shares - member.reserved_shares;
            ensure!(
                to_burn =< free_shares
                "not enough free shares to burn"
            );
            // TODO: priority to verify valid behavior below via tests
            let burn_ratio = Permill::from_rational_approximation(to_burn, Self::total_shares());
            let proportionate_balance = burn_ratio * Self::pot();
            // NOTE: if the above doesn't work, multiple numerator in `from_rational_approximation` by Self::pot()
            let _ = T::Currency::transfer(&Self::account_id(), &arsonist, proportionate_balance)?;
            if to_burn < free_shares {
                // member is only burning some of their shares
                member.all_shares -= to_burn;
            } else {
                ensure!(!<Voter<T>>::exists(&arsonist), "get voter bond (deregister) before leaving the DAO");
                // member is burning all shares and leaving the DAO
                <Member<T>>::mutate(|members| members.retain(|m| m != &arsonist));
                <MemberInfo<T>>::remove(&arsonist);
            }
            <TotalShares<T>>::mutate(|val| val -= to_burn);
            Self::deposit_event(RawEvent::SharesBurned(arsonist, to_burn, proportionate_balance));
            Ok(())
        }

        fn on_finalize(n: T::BlockNumber) {
            // TODO: frequency adjustments based on proposal throughput

            // PURGE
            if (n % T::SweepFrequency::get()).is_zero() {
                Self::sweep();
            }

            // ISSUANCE
            if (n % T::IssuanceFrequency::get()).is_zero() {
                Self::issuance(n);
            }// TODO: is_zero() requires the Zero trait from runtime_primitives::traits

            Self::deposit_event(RawEvent::BudgetRemaining(budget));
        }
    }
}

// test if order matters
decl_storage! {
    trait Store for Module<T: Trait> as MiniDAO {
        /// Applications that are awaiting sponsorship
        Applications get(applications): map T::Hash => Application<T::AccountId, BalanceOf<T>, T::BlockNumber>;
        /// All applicants
        Applicant get(applicant): Vec<T::AccountId>;
        /// Applications that have grown stale, awaiting purging
        StaleApps get(stale_apps): Vec<T::Hash>;

        /// Proposals that have been made.
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>>>;
        /// From Proposal Hash to election state
        Elections get(elections): map T::Hash => Election<T::AccountId, T::BlockNumber>;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        StaleProposals get(stale_proposals): Vec<T::Hash>;

        // Outstanding shares requested (directly proposed || apply + sponsor)
        SharesRequested get(shares_requested): Shares;
        // Total shares issued
        TotalShares get(total_issued): Shares;

        /// Members of the DAO
        Members get(members): Vec<T::AccountId>;
        /// Tracking membership shares (voting strength)
        MemberInfo get(member_info): map T::AccountId => Member<BalanceOf<T>>;
        /// Active voting members (requires additional bond)
        Voters get(voters): Vec<T::AccountId>;

        // TODO: re-add proxy functionality; consider alternative APIs
    }
}

impl<T: Trait> Module<T> {
    // ----DAO----
    pub fn account_id() -> T::AccountId {
        DAO_ID.into_account()
    } // TODO: requires trait AccountIdConversion

    // total funds in DAO
    fn pot() -> BalanceOf<T> {
        T::Currency::free_balance(&Self::account_id())
    }

    // ----Supporters for Ensure Checks----
    pub fn is_applicant(who: &T::AccountId) -> bool {
        Self::applicant().contains(who)
    }
    pub fn is_member(who: &T::AccountId) -> bool {
        Self::members().contains(who)
    }
    pub fn is_voter(who: &T::AccountId) -> bool {
        Self::voter().contains(who)
    }

    // ----Bond Calculations----
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
    fn vote_bond(new_voter: T::AccountId) -> BalanceOf<T> {
        // TODO: make more flexible, based on outstanding voter stats
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

    // ----Auxiliary Methods----
    fn reserve_shares(voter: T::AccountId, support: Shares) -> Result {
        let mut member = Self::member_info(&voter).ok_or("voter must be a member")?;
        let free_shares = member.all_shares - member.reserved_shares;
        ensure!(support <= free_shares, "not enough free shares to signal support during sponsor");
        Self::member_info::mutate(&member, |m| m.reserved_shares += support);
        Ok(())
    }
    // Enables burning of more shares based on proposal passage typically or passage
    // but the voter dissented and is freed during the GraceWindow
    fn liberate(free_voters: &[T::AccountId]) -> Result {
        // taking input as slice is optimal; prefer to iterate over slice vs vec
        free_voters.into_iter().for_each(|vote| {
            let mut voter = Self::member_info(vote.0).ok_or("voter does not exist")?;
            // shares are returned because this side lost
            voter.reserved_shares -= vote.1;
        });
        Ok(())
    }

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
        T::Currency::reserve(&sponsor, sponsor_bond).map_err(|_| "Proposer's balance too low")?;

        let threshold = Self::calculate_threshold(shares_requested, donation);

        Self::reserve_shares(sponsor.clone(), support)?;

        // start the voting period
        let vote_start = <system::Module<T>>::block_number();
        let grace_end = None;

        // ayes and nays vectors
        let mut ayes = Vec::new();
        let mut nays: Vec<(T::AccountId, Shares)> = Vec::new();
        ayes.push((sponsor.clone(), support));

        // clone proposer for event emission after proposal insertion
        let s = sponsor.clone();
        let proposal = Proposal {
            sponsor,
            sponsor_bond,
            applicant,
            donation,
            shares_requested,
        };
        let election = Election {
            applicant, // might require a clone
            vote_start,
            grace_end,
            threshold,
            ayes,
            nays,
        };
        // take hash of proposal
        let hash = <T as system::Trait>::Hashing::hash_of(&proposal);
        // insert proposal
        Self::proposals::insert(hash, &proposal);
        // insert election
        Self::elections::insert(hash, &election);
        Self::deposit_event(RawEvent::Proposed(s, hash, shares_requested, vote_start));
        Ok(())
    }

    /// Voting
    ///
    /// Called by `vote` and `proxy_vote`
    fn do_vote(voter: T::AccountId, hash: T::Hash, support: Shares, approve: bool) -> Result {
        // verify election existence
        let mut election = Self::elections(&hash).ok_or("election must exit")?;
        ensure!(
            election.applicant != &voter,
            "potential recipient may not vote"
        );

        // check if the proposal has already passed and entered the grace period
        if let Some(time) = election.grace_end {
            return Err("Proposal passed, grace period already started");
        }

        let now = <system::Module<T>>::block_number();

        if election.vote_start + T::VoteWindow::get() < now {
            Self::stale_proposals::mutate(|proposals| proposals.push(hash.clone())); // TODO: update with `append` once PR merged
            return Err("Proposal is stale, no more voting!");
        }

        let position_yes = election.ayes.iter().position(|a| a.0 == &voter);
        let position_no = election.nays.iter().position(|a| a.0 == &voter);

        if approve {
            if position_yes.is_none() {
                election.ayes.push((voter.clone(), support));
                election.state.0 += support;
            } else {
                return Err("duplicate vote");
            }
            // executes if the previous vote was no
            if let Some(pos) = position_no {
                // ability to change vote at no cost prevents bribery attacks
                election.nays.swap_remove(pos);
                election.state.1 -= pos.1;
                election.state.0 += pos.1;
            }
        } else {
            if position_no.is_none() {
                election.nays.push((voter.clone(), support));
                election.state.1 += support;
            } else {
                return Err("duplicate vote");
            }
            if let Some(pos) = position_yes {
                election.ayes.swap_remove(pos);
                election.state.0 -= pos.1;
                election.state.1 += pos.1;
            }
        }
        // update MemberInfo to reflect vote reservations
        Self::reserve_shares(voter.clone(), support)?;
        // unnecessary type aliasing
        let total_support = election.state.0;
        let total_against = election.state.1;
        // TODO: election.threshold should be updated if surrounding state has changed?
        Self::deposit_event(RawEvent::Voted(
            voter,
            now,
            hash,
            approve,
            total_support,
            total_against,
        ));

        if total_support > election.threshold {
            Self::passed::mutate(|pass| pass.push(hash)); // TODO: update with `append` once 3071 merged
            election.grace_end = now + T::GraceWindow::get();
            // pass a slice of AccountIds for dissenting voters
            Self::liberate(&election.nays[..]);
        }

        // TODO: add path in which negative feedback rises above some threshold
        // should slash some of the proposal bond in this event
        Ok(())
    }

    /// Execution
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn issuance(n: T::BlockNumber) -> Result {
        Self::passed::get().into_iter().for_each(|&hash| {
            if let Some(e) = Self::elections(hash) {
                // liberal grace period (sometimes longer than necessary)
                if n >= e.grace_end {
                    if let Some(p) = Self::proposals(hash) {
                        // unbond the sponsor
                        T::Currency::unreserve(&p.sponsor, &p.sponsor_bond);
                        // transfer donation from applicant
                        T::Currency::unreserve(&p.applicant, &p.donation);
                        T::Currency::transfer(&p.applicant, &Self::account_id(), &p.donation);

                        if Self::members::exists(&p.applicant) {
                            Self::member_info::mutate(&p.applicant, |mem| {
                                mem.all_shares += &p.shares_requested
                            });
                        } else {
                            let all_shares = &p.shares_requested;
                            let reserved_shares = 0 as Shares;
                            let voter_bond = None;
                            Self::member_info::insert(
                                &p.applicant,
                                Member {
                                    all_shares,
                                    reserved_shares,
                                    voter_bond,
                                },
                            );
                            Self::members::mutate(|mems| mems.push(p.applicant.clone())); // `append` with 3071
                        }
                        Self::shares_requested::mutate(|valu| valu -= &p.shares_requested);
                        Self::total_shares::mutate(|val| val += &p.shares_requested);
                    }
                    Self::liberate(&e.ayes[..]);
                }
            }
            Self::elections::remove(hash);
            Self::proposals::remove(hash);
        });
        Self::passed::kill();
        Ok(())
    }

    /// Purging stale proposals
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn sweep() {
        // use type state to consume lazy iterator adaptor
        let _ = <StaleProposals<T>>::get().into_iter().for_each(|h| {
            if let Some(election) = Self::elections(&h) {
                let nays = election.nays.clone();
                let old_votes = election.ayes.append(&mut nays);
                Self::liberate(&old_votes[..]);
            }
            let proposal = Self::proposals(&h).ok_or("proposal dne")?;
            Self::shares_requested::mutate(|val| val -= proposal.shares_requested);
            // negative incentives, proposal bond goes to the treasury
            T::Currency::unreserve(&proposal.sponsor, &proposal.sponsor_bond);
            T::Currency::transfer(&proposal.sponsor, &Self::account_id(), &proposal.sponsor_bond);
            Self::elections::remove(&h);
            Self::proposals::remove(&h);
        });
        Self::stale_proposals::kill();

        let _ = Self::stale_apps::get().into_iter().for_each(|h| {
            // // uncomment to add punishment
            // if let Some(app) = Self::applications(&h) {
            //     // punish by taking percent of donation here
            // }
            Self::applications::remove(&h);
        });
        Self::stale_apps::kill();
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
