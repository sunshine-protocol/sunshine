use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use runtime_primitives::traits::{AccountIdConversion, Hash, Zero}; // StaticLookup
use runtime_primitives::{ModuleId, Permill};
use support::traits::{Currency, Get, ReservableCurrency};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// START TYPES
type Shares = u32;
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
    applier: AccountId,
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

    /// Bond required for proposal
    type ProposalBond: Get<BalanceOf<Self>>;

    /// Bond required to become an active voter
    type VoteBond: Get<BalanceOf<Self>>;

    /// Period for which applications are valid after initial application
    type ApplicationWindow: Get<Self::BlockNumber>;

    /// Period during which applications can be revoked without penalty
    type AbortWindow: Get<Self::BlockNumber>;

    /// Period for which votes are valid after initial proposal
    type VoteWindow: Get<Self::BlockNumber>;

    /// Period after proposal acceptance during which *commitment-free* dissenters may exit
    type GraceWindow: Get<Self::BlockNumber>;

    /// Frequency with which stale proposals and applications are swept
    /// -- purpose: mitigate state bloat
    type SweepFrequency: Get<Self::BlockNumber>;

    /// Frequency with which the passed proposals are executed
    type IssuanceFrequency: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        <T as system::Trait>::Hash,
        Shares = Shares,
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
        Voted(AccountId, BlockNumber, Hash, bool, Shares, Shares), // TODO: add BlockNumber
        // new member AccountId, shares issued, balance committed
        SharesIssued(AccountId, Shares, Balance),
        // member ID, shares burned, balance returned
        SharesBurned(AccountId, Shares, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as DAO {
        /// Applications that are awaiting sponsorship
        Applications get(applications): map T::Hash => Option<Application<T::AccountId, BalanceOf<T>, T::BlockNumber>>;
        /// All applicants
        Applicants get(applicants): Vec<T::AccountId>;
        /// Applications that have grown stale, awaiting purging
        StaleApps get(stale_apps): Vec<T::Hash>;

        /// Proposals that have been made.
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>>>;
        /// From Proposal Hash to election state
        Elections get(elections): map T::Hash => Option<Election<T::AccountId, T::BlockNumber>>;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        StaleProposals get(stale_proposals): Vec<T::Hash>;

        // Outstanding shares requested (directly proposed || apply + sponsor)
        SharesRequested get(shares_requested): Shares;
        // Total shares issued
        TotalShares get(total_shares): Shares;

        /// Members of the DAO
        Members get(members): Vec<T::AccountId>;
        /// Tracking membership shares (voting strength)
        MemberInfo get(member_info): map T::AccountId => Option<Member<BalanceOf<T>>>;
        /// Active voting members (requires additional bond)
        Voters get(voters): Vec<T::AccountId>;

        // TODO: re-add proxy functionality; consider alternative APIs
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Bond for proposals, to be returned
        const ProposalBond: BalanceOf<T> = T::ProposalBond::get();

        /// Bond for active voters, to be returned
        const VoteBond: BalanceOf<T> = T::VoteBond::get();

        /// Period in which applications are valid after initial application
        const ApplicationWindow: T::BlockNumber = T::ApplicationWindow::get();

        /// Period during which applications can be revoked without penalty
        const AbortWindow: T::BlockNumber = T::AbortWindow::get();

        /// Period after initial proposal during which voting is allowed
        const VoteWindow: T::BlockNumber = T::VoteWindow::get();

        /// Period after proposal passage during which *commitment-free* dissents can exit early
        const GraceWindow: T::BlockNumber = T::GraceWindow::get();

        /// Period between stale proposal purges
        const SweepFrequency: T::BlockNumber = T::SweepFrequency::get();

        /// Period between successive spends.
        const IssuanceFrequency: T::BlockNumber = T::IssuanceFrequency::get();

        fn deposit_event<T>() = default;

        fn apply(origin, shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Result {
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
                shares_requested,
                start,
            };
            // take hash of application
            let hash = <T as system::Trait>::Hashing::hash_of(&app);
            // insert application
            <Applications<T>>::insert(hash, &app);
            // add applicant to applicant pool
            <Applicants<T>>::mutate(|applicants| applicants.push(a.clone())); // TODO: `append` once 3071
            // deposit event
            Self::deposit_event(RawEvent::Applied(a, hash, donation, shares_requested));
            Ok(())
        }

        fn sponsor(origin, app_hash: T::Hash, support: Shares) -> Result {
            let sponsor = ensure_signed(origin)?;
            ensure!(Self::is_member(&sponsor), "sponsor must be a member");

            let app = Self::applications(&app_hash).ok_or("application must exist")?;
            // if the sponsor is the applicant, there are required restrictions on voting for self (see `fn propose`)
            ensure!(&app.applicant != &sponsor, "direct proposals should be made via `propose`");

            // TODO: better system for managing pending applications (better purge)
            if app.start + T::ApplicationWindow::get() < <system::Module<T>>::block_number() {
                <StaleApps<T>>::mutate(|s| s.push(app_hash.clone())); // TODO: `append` after 3071 merged
                return Err("The window has passed for this application");
            }

            // make the proposal
            Self::do_propose(sponsor.clone(), app.applicant, support, app.donation, app.shares_requested)?;

            // remove the application, the proposal is live
            <Applications<T>>::remove(app_hash);

            Ok(())
        }

        /// Direct proposals made by members for members (no apply + sponsor)
        ///
        /// This is a special case in which an existing member would like to request more shares
        /// -- the `calculate_threshold` must take into account the support given by this member and offset it
        /// --
        fn propose(origin, shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Result {
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
            Self::do_propose(proposer.clone(), proposer, 0, donation, shares_requested)?;
            Ok(())
        }

        fn abort(origin, hash: T::Hash) -> Result {
            let abortee = ensure_signed(origin)?;

            let now = <system::Module<T>>::block_number();

            // if application, purge application and end
            if let Some(app) = Self::applications(&hash) {
                ensure!(&abortee == &app.applicant, "only the applicant can abort the proposal");
                if app.start + T::ApplicationWindow::get() < now {
                    <StaleApps<T>>::mutate(|s| s.push(hash.clone())); // TODO: `append` after 3071 merged
                    return Err("The window has passed for this application");
                }

                <Applications<T>>::remove(&hash);
            }

            // if live proposal, cleanup is harder are necessary
            if let Some(mut election) = Self::elections(&hash) {
                // only the applicant can call abort
                ensure!(&abortee == &election.applier, "only the applicant can abort the proposal");

                // no aborting if the proposal has overcome the threshold of support for/against
                ensure!(election.threshold > election.state.0 || election.threshold > election.state.1, "too controversial; the vote already has too much support for/against the proposal");

                // checking against time
                if election.vote_start + T::AbortWindow::get() < now {
                    return Err("past the abort window, too late");
                } else if election.vote_start + T::VoteWindow::get() < now {
                    <StaleProposals<T>>::mutate(|proposals| proposals.push(hash.clone())); // TODO: update with `append` once PR merged
                    return Err("Proposal is stale, no more voting!");
                } else if let Some(time) = election.grace_end {
                    return Err("Proposal passed, grace period already started");
                }

                // liberate voters
                let votes: Vec<(T::AccountId, Shares)> = election.ayes.into_iter().chain(election.nays).collect();
                Self::liberate(&votes[..]);
            }

            if let Some(proposal) = Self::proposals(&hash) {
                let shares_requested = <SharesRequested>::get() - proposal.shares_requested;
                <SharesRequested>::put(shares_requested);
                // negative incentives, proposal bond goes to the treasury
                T::Currency::unreserve(&proposal.sponsor, proposal.sponsor_bond);
                T::Currency::transfer(&proposal.sponsor, &Self::account_id(), proposal.sponsor_bond);
            }
            <Elections<T>>::remove(&hash);
            <Proposals<T>>::remove(&hash);

            Ok(())
        }

        // register as a new voter
        fn register(origin) -> Result {
            let new_voter = ensure_signed(origin)?;
            ensure!(Self::is_member(&new_voter), "every voter must be a member");
            ensure!(!Self::is_voter(&new_voter), "must not be an active voter yet");

            let vote_bond = Self::vote_bond();
            T::Currency::reserve(&new_voter, vote_bond)
                .map_err(|_| "member doesn't have enough free balance for vote bond")?;

            <Voters<T>>::mutate(|v| v.push(new_voter.clone())); // `append` once 3071 merged
            let start = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::RegisterVoter(new_voter, start));
            Ok(())
        }

        // deregister as a new voter
        fn deregister(origin) -> Result {
            let old_voter = ensure_signed(origin)?;
            ensure!(Self::is_voter(&old_voter), "must be an active voter");

            // TODO: measure instances of misbehavior in a stateful way; slash some percent here
            T::Currency::unreserve(&old_voter, T::VoteBond::get());

            <Voters<T>>::mutate(|voters| voters.retain(|v| v != &old_voter));
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
        pub fn burn(origin, to_burn: Shares) -> Result {
            ensure!(to_burn > 0, "can only burn > 0 shares");
            let arsonist = ensure_signed(origin)?;
            let mut member = Self::member_info(&arsonist).ok_or("must be a member to burn shares")?;
            let free_shares = member.all_shares - member.reserved_shares;
            ensure!(to_burn <= free_shares, "not enough free shares to burn");
            // TODO: priority to verify valid behavior below via tests
            let burn_ratio = Permill::from_rational_approximation(to_burn, Self::total_shares());
            let proportionate_balance = burn_ratio * Self::pot();
            // NOTE: if the above doesn't work, multiple numerator in `from_rational_approximation` by Self::pot()
            let _ = T::Currency::transfer(&Self::account_id(), &arsonist, proportionate_balance)?;
            if to_burn < free_shares {
                // member is only burning some of their shares
                member.all_shares -= to_burn;
            } else {
                ensure!(!Self::is_voter(&arsonist), "get voter bond (deregister) before leaving the DAO");
                // member is burning all shares and leaving the DAO
                <Members<T>>::mutate(|members| members.retain(|m| m != &arsonist));
                <MemberInfo<T>>::remove(&arsonist);
            }
            let total_shares_issued = Self::total_shares() - to_burn;
            <TotalShares>::put(total_shares_issued);
            Self::deposit_event(RawEvent::SharesBurned(arsonist, to_burn, proportionate_balance));
            Ok(())
        }

        fn on_finalize(n: T::BlockNumber) {
            // TODO: frequency adjustments based on proposal throughput (`Rolling Window`)

            // PURGE
            if (n % T::SweepFrequency::get()).is_zero() {
                Self::sweep();
            }

            // ISSUANCE
            if (n % T::IssuanceFrequency::get()).is_zero() {
                Self::issuance(n);
            }// TODO: is_zero() requires the Zero trait from runtime_primitives::traits
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
    pub fn is_member(who: &T::AccountId) -> bool {
        Self::members().contains(who)
    }
    pub fn is_applicant(who: &T::AccountId) -> bool {
        Self::applicants().contains(who)
    }
    pub fn is_voter(who: &T::AccountId) -> bool {
        Self::voters().contains(who)
    }

    fn proposal_bond(shares_requested: Shares, donation: Option<BalanceOf<T>>) -> BalanceOf<T> {
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
    fn vote_bond() -> BalanceOf<T> {
        // TODO: make more flexible, based on outstanding voter stats
        let temp_to_change = Permill::from_percent(10) * Self::pot();

        // add to member voter bond information
        temp_to_change
    }
    fn calculate_threshold(shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Shares {
        // TODO: should depend on inputs, total outstanding requests, and other factors (like the proposal_bond)

        // temporary hard-coded threshold (> 20% of outstanding shares)
        let total_shares = Self::total_shares();
        let threshold = total_shares / 5 + 1;
        threshold
    }

    // ----Auxiliary Methods----
    fn reserve_shares(voter: T::AccountId, support: Shares) -> Result {
        let mut member = <MemberInfo<T>>::get(&voter).ok_or("voter must be a member")?;
        let free_shares = member.all_shares - member.reserved_shares;
        ensure!(
            support <= free_shares,
            "not enough free shares to signal support during sponsor"
        );
        member.reserved_shares += support;
        Ok(())
    }
    // Enables burning of more shares based on proposal passage typically or passage
    // but the voter dissented and is freed during the GraceWindow
    fn liberate(free_voters: &[(T::AccountId, Shares)]) -> Result {
        // taking input as slice is optimal; prefer to iterate over slice vs vec
        let _ = free_voters.into_iter().for_each(|(voter_id, support)| {
            if let Some(mut v) = <MemberInfo<T>>::get(voter_id) {
                v.reserved_shares -= support;
            } // TODO: alert/notify if there is no member info (=> corrupted storage)
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
        let state: (Shares, Shares) = (support, 0);

        // clone proposer for event emission after proposal insertion
        let s = sponsor.clone();
        let applier = applicant.clone();
        let proposal = Proposal {
            sponsor,
            sponsor_bond,
            applicant,
            donation,
            shares_requested,
        };
        let election = Election {
            applier, // might require a clone
            vote_start,
            grace_end,
            threshold,
            state,
            ayes,
            nays,
        };
        // take hash of proposal
        let hash = <T as system::Trait>::Hashing::hash_of(&proposal);
        // insert proposal
        <Proposals<T>>::insert(hash, &proposal);
        // insert election
        <Elections<T>>::insert(hash, &election);
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
            &election.applier != &voter,
            "potential recipient may not vote"
        );

        // check if the proposal has already passed and entered the grace period
        if let Some(time) = election.grace_end {
            return Err("Proposal passed, grace period already started");
        }

        let now = <system::Module<T>>::block_number();

        if election.vote_start + T::VoteWindow::get() < now {
            <StaleProposals<T>>::mutate(|proposals| proposals.push(hash.clone())); // TODO: update with `append` once PR merged
            return Err("Proposal is stale, no more voting!");
        }

        let position_yes = election.ayes.iter().position(|a| &a.0 == &voter);
        let position_no = election.nays.iter().position(|a| &a.0 == &voter);

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
                election.state.0 += support;
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
                election.state.1 += support;
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
            <Passed<T>>::mutate(|pass| pass.push(hash)); // TODO: update with `append` once 3071 merged
            election.grace_end = Some(now + T::GraceWindow::get());
            // pass a slice of AccountIds for dissenting voters
            let _ = Self::liberate(&election.nays[..]);
        }

        // TODO: add path in which negative feedback rises above some threshold
        // should slash some of the proposal bond under this event
        Ok(())
    }

    /// Execution
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn issuance(n: T::BlockNumber) -> Result {
        <Passed<T>>::get().into_iter().for_each(|hash| {
            if let Some(e) = Self::elections(hash) {
                // liberal grace period (sometimes longer than necessary)
                if let Some(end) = e.grace_end {
                    if end <= n {
                        if let Some(p) = Self::proposals(hash) {
                            // unbond the sponsor
                            T::Currency::unreserve(&p.sponsor, p.sponsor_bond);
                            if let Some(donate) = p.donation {
                                // transfer donation from applicant
                                T::Currency::unreserve(&p.applicant, donate);
                                T::Currency::transfer(&p.applicant, &Self::account_id(), donate);
                            }
                            if Self::is_member(&p.applicant) {
                                <MemberInfo<T>>::mutate(&p.applicant, |mem| {
                                    if let Some(memb) = mem {
                                        memb.all_shares += p.shares_requested;
                                    }
                                });
                            } else {
                                let all_shares = p.shares_requested;
                                let reserved_shares = 0 as Shares;
                                let voter_bond = None;
                                <MemberInfo<T>>::insert(
                                    &p.applicant,
                                    Member {
                                        all_shares,
                                        reserved_shares,
                                        voter_bond,
                                    },
                                );
                                <Members<T>>::mutate(|mems| mems.push(p.applicant.clone())); // `append` with 3071
                            }
                            let shares_requested = <SharesRequested>::get() - p.shares_requested;
                            <SharesRequested>::put(shares_requested);
                            let total_shares = <TotalShares>::get() + p.shares_requested; // TODO: checked_add
                            <TotalShares>::put(total_shares);
                        }
                        Self::liberate(&e.ayes[..]);
                    }
                }
            }
            <Elections<T>>::remove(hash);
            <Proposals<T>>::remove(hash);
        });
        <Passed<T>>::kill();
        Ok(())
    }

    /// Purging stale proposals
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn sweep() {
        // use type state to consume lazy iterator adaptor
        let _ = <StaleProposals<T>>::get().into_iter().for_each(|h| {
            if let Some(election) = Self::elections(&h) {
                let votes: Vec<(T::AccountId, Shares)> =
                    election.ayes.into_iter().chain(election.nays).collect();
                Self::liberate(&votes[..]);
            }
            if let Some(proposal) = Self::proposals(&h) {
                let shares_requested = <SharesRequested>::get() - proposal.shares_requested;
                <SharesRequested>::put(shares_requested);
                // negative incentives, proposal bond goes to the treasury
                T::Currency::unreserve(&proposal.sponsor, proposal.sponsor_bond);
                T::Currency::transfer(
                    &proposal.sponsor,
                    &Self::account_id(),
                    proposal.sponsor_bond,
                );
            }
            <Elections<T>>::remove(&h);
            <Proposals<T>>::remove(&h);
        });
        <StaleProposals<T>>::kill();

        let _ = <StaleApps<T>>::get().into_iter().for_each(|h| {
            // // uncomment to add punishment
            // if let Some(app) = Self::applications(&h) {
            //     // punish by taking percent of donation here
            // }
            <Applications<T>>::remove(&h);
        });
        <StaleApps<T>>::kill();
    }
}

// TESTING
#[cfg(test)]
mod tests {
    use crate::tests::Origin;
    use crate::tests::*; // {Call, Event as OuterEvent}
                         // use runtime_primitives::{
                         //     testing::Header,
                         //     traits::{BlakeTwo256, IdentityLookup, OnFinalize},
                         // };
    use support::{assert_err, assert_noop, assert_ok}; // dispatch::Result, Hashable
    use system::ensure_signed; // there must be a better way of getting AccountId

    #[test]
    fn basic_setup_works() {
        with_externalities(&mut new_test_ext(), || {
            assert_eq!(DAO::pot(), 0);
            assert_eq!(DAO::total_proposals(), 0);
        });
    }

    #[test]
    fn join_works() {
        with_externalities(&mut new_test_ext(), || {
            // test that join takes 10 from balance
            assert_ok!(DAO::join(Origin::signed(0)));
            assert_eq!(Balances::free_balance(&0), 90);
            // get address for checking membership
            let who = ensure_signed(Origin::signed(0)).expect("smh^smh");
            assert!(DAO::is_member(&who));; // how do I get the accountId
                                            // join request from existing member should fail
            assert_noop!(
                DAO::join(Origin::signed(0)),
                "new member is already a member"
            );
            // (3, 9) can't join because 9 < 10 (and 10 is EntryFee)
            assert_noop!(DAO::join(Origin::signed(3)), "Not rich enough to join ;(");
        });
    }

    #[test]
    fn exit_works() {
        with_externalities(&mut new_test_ext(), || {
            // join to exit immediately after
            assert_ok!(DAO::join(Origin::signed(0)));
            // exit should work
            assert_ok!(DAO::exit(Origin::signed(0)));
            // exit for non-member should not work
            assert_noop!(
                DAO::exit(Origin::signed(1)),
                "exiting member must be a member"
            );
        });
    }

    #[test]
    fn propose_works() {
        with_externalities(&mut new_test_ext(), || {
            // nonmember propose fails
            assert_noop!(
                DAO::propose(Origin::signed(0), 10, 3),
                "proposer must be a member to make a proposal"
            );
            // join to add 10 to the treasury
            assert_ok!(DAO::join(Origin::signed(0)));
            // proposal outweighs DAO's funds
            assert_noop!(
                DAO::propose(Origin::signed(0), 11, 3),
                "not enough funds in the DAO to execute proposal"
            );
            // 10 + 10 = 20
            assert_ok!(DAO::join(Origin::signed(1)));
            // proposal should work
            assert_ok!(DAO::propose(Origin::signed(0), 11, 3));
            assert_eq!(DAO::total_proposals(), 1);
            // 100 - EntryFee(10) - ProposalBond(2) = 88
            assert_eq!(Balances::free_balance(&0), 88);
            // proposal can't be done without proposal bond
            assert_ok!(DAO::join(Origin::signed(4)));
            assert_noop!(
                DAO::propose(Origin::signed(4), 10, 3),
                "Proposer's balance too low"
            );
        });
    }

    #[test]
    fn vote_works() {
        with_externalities(&mut new_test_ext(), || {
            // nonmember can't vote
            assert_noop!(
                DAO::vote(Origin::signed(0), 1, true),
                "voter must be a member to approve/deny a proposal"
            );
            // join, join
            assert_ok!(DAO::join(Origin::signed(0)));
            assert_ok!(DAO::join(Origin::signed(1)));
            // can't vote on nonexistent proposal
            assert_noop!(DAO::vote(Origin::signed(1), 1, true), "proposal must exist");
            // make proposal for voting
            assert_ok!(DAO::propose(Origin::signed(0), 11, 3));
            assert_eq!(DAO::total_proposals(), 1);
            // vote for member works
            assert_ok!(DAO::vote(Origin::signed(1), 1, true));
            // can't duplicate vote
            // assert_noop!(DAO::vote(Origin::signed(1), 0, true), "duplicate vote"); // doesn't really work
            // can switch vote
            assert_ok!(DAO::vote(Origin::signed(1), 1, false));
            // can't duplicate vote
            // assert_noop!(DAO::vote(Origin::signed(1), 0, false), "duplicate vote"); // doesn't really work
        });
    }
}
