/// Membership Module
/// -- share-weighted voting
/// -- collateral from applicants is `Balance`; collateral from voters/proposers is `Shares`
/// -- implements lock-in and *instant withdrawal* like Moloch
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use runtime_primitives::traits::{AccountIdConversion, Hash, Zero}; // StaticLookup
use runtime_primitives::{ModuleId, Permill};
use support::traits::{Currency, Get, ReservableCurrency};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// START TYPES
type Shares = u32; // TODO: use traits::shares instead of this (and use as module type)
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

/// Generic Moloch Proposal
///
/// TODO: replace Threshold with struct-based voting algorithm object
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, Balance, BlockNumber> {
    // the proposal sponsor must be a member
    sponsor: AccountId,
    // the sponsor's vote (/bond in support, like a voter)
    sponsor_vote: Shares,
    // the applicant does not need to be a member
    applicant: Option<AccountId>,
    // applicant's donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,
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
pub struct Member {
    // total shares owned by the member
    all_shares: Shares,
    // shares reserved based on pending proposal support
    reserved_shares: Shares,
    // shares reserved for voters
    voter_bond: Option<Shares>,
    // TODO: consider more stateful pattern via `BTreeMap<Hash, Shares>` for vote pairs
}
// END TYPES

pub trait Trait: system::Trait {
    /// The balances type
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Bond required for proposal
    type ProposalBond: Get<Shares>; // TODO: add issue and change this bond to shares

    /// Bond required to become an active voter
    type VoteBond: Get<Shares>; // TODO: add issue and change this bond to shares

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
        // applicant AccountId, application Hash, donation amount (if any), shares requested
        Applied(AccountId, Hash, Option<Balance>, Shares),
        // applicant AccountId, application Hash, 
        Aborted(AccountId, Hash), // TODO: could add abortion fee charged (if any); atm \exists none
        // sponsor AccountId, proposal Hash, shares requested, block number
        Proposed(AccountId, Hash, Shares, BlockNumber),
        // new voter AccountId, Shares bonded by new voter, BlockNumber at registration
        RegisterVoter(AccountId, Shares, BlockNumber),
        // old voter AccountId, BlockNumber at deregistration
        DeRegisterVoter(AccountId, Shares, BlockNumber),
        // voter AccountId, block number, proposal Hash, vote bool, vote_weight, yes_count, no_count
        Voted(AccountId, BlockNumber, Hash, bool, Shares, Shares, Shares), // TODO: add BlockNumber
        // new member AccountId, shares issued, donation (balance) staked
        SharesIssued(AccountId, Shares, Balance),
        // member ID, shares burned, funds withdrawn from exit
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
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber>>;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        StaleProposals get(stale_proposals): Vec<T::Hash>;

        // Outstanding shares requested (directly proposed || apply + sponsor)
        SharesRequested get(shares_requested): Shares;
        // Total shares issued
        TotalShares get(total_shares): Shares;

        // TotalShares get(total_shares) build(|config: &GenesisConfig<T>| {
        //     config.memberz.iter().fold(Zero::zero(), |acc: Shares, &(_, n)| acc + n)
        // }): Shares;

        /// Members of the DAO
        Members get(members): Vec<T::AccountId>;
        /// Tracking membership shares (voting strength)
        MemberInfo get(member_info): map T::AccountId => Option<Member>;
        /// Active voting members (requires addition Shares-based bond)
        Voters get(voters): Vec<T::AccountId>;
    }
    // add_extra_genesis {} // for tests set up
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        pub fn deposit_event() = default;
        /// Bond for proposals, to be returned
        const ProposalBond: Shares = T::ProposalBond::get();

        /// Bond for active voters, to be returned
        const VoteBond: Shares = T::VoteBond::get();

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

        fn apply(origin, shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Result {
            let applicant = ensure_signed(origin)?;
            // if removed, we risk duplicate applications with the same hash?
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
            Self::do_propose(sponsor, Some(app.applicant), support, app.donation, app.shares_requested)?;

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
            Self::do_propose(proposer, None, 0, donation, shares_requested)?;
            Ok(())
        }

        fn abort(origin, hash: T::Hash) -> Result {
            let abortee = ensure_signed(origin)?;

            let now = <system::Module<T>>::block_number();

            // if application, purge application and end
            if let Some(app) = Self::applications(&hash) {
                // INVARIANT: an application must only exist iff there are no associated proposals
                // -- this prevents unreserving the donation amount twice inadvertently
                ensure!(&abortee == &app.applicant, "only the applicant can abort the proposal");
                if app.start + T::ApplicationWindow::get() < now {
                    <StaleApps<T>>::mutate(|s| s.push(hash.clone())); // TODO: `append` after 3071 merged
                    return Err("The window has passed for this application");
                }
                // return the application donation
                if let Some(donate) = app.donation {
                    T::Currency::unreserve(&app.applicant, donate);
                    // TODO: could calculate and charge an abort fee here
                }

                <Applications<T>>::remove(&hash);
                Self::deposit_event(RawEvent::Aborted(abortee.clone(), hash.clone()));
                return Ok(());
            }

            // if live proposal, cleanup is harder are necessary
            if let Some(proposal) = Self::proposals(&hash) {
                // only the applicant can call abort // TODO: => direct proposals cannot be aborted? rethink this...
                ensure!(proposal.applicant.is_some(), "only the applicant can abort the proposal");

                // no aborting if the proposal has overcome the threshold of support for/against
                ensure!(proposal.threshold > proposal.state.0 || proposal.threshold > proposal.state.1, "too controversial; the vote already has too much support for/against the proposal");

                // checking valid time
                ensure!(proposal.grace_end.is_none(), "Proposal passed, grace period already started");
                if proposal.vote_start + T::AbortWindow::get() < now {
                    return Err("past the abort window, too late");
                }

                // subtract requested share count
                let shares_requested = <SharesRequested>::get() - proposal.shares_requested;
                <SharesRequested>::put(shares_requested);

                // liberate voters
                let votes: Vec<(T::AccountId, Shares)> = proposal.ayes.into_iter().chain(proposal.nays).collect();
                Self::liberate(&votes[..]);
            }

            <Proposals<T>>::remove(&hash);
            Self::deposit_event(RawEvent::Aborted(abortee, hash));
            Ok(())
        }

        // register as a new voter
        fn register(origin) -> Result {
            let new_voter = ensure_signed(origin)?;

            let mut member = <MemberInfo<T>>::get(&new_voter).ok_or("no info on member!")?;
            ensure!(!Self::is_voter(&new_voter), "must not be an active voter yet");

            let vote_bond: Shares = Self::calculate_vbond();
            member.voter_bond = Some(vote_bond);

            <Voters<T>>::mutate(|v| v.push(new_voter.clone())); // `append` once 3071 merged
            let start = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::RegisterVoter(new_voter, vote_bond, start));
            Ok(())
        }

        /// Deregister an active voter
        ///
        /// -- returns `voter_bond`
        /// -- prevents any voting upon execution
        fn deregister(origin) -> Result {
            let old_voter = ensure_signed(origin)?;
            ensure!(Self::is_voter(&old_voter), "must be an active voter");

            let mut member = <MemberInfo<T>>::get(&old_voter).ok_or("no member information")?;
            let mut shares_released: Shares = 0;
            if let Some(bond_remains) = member.voter_bond {
                member.voter_bond = None; // return the bond
                shares_released += bond_remains;
            }

            // take the voter out of the associated storage item (prevent future voting)
            <Voters<T>>::mutate(|voters| voters.retain(|v| v != &old_voter));
            let end = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::DeRegisterVoter(old_voter, shares_released, end));
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
            let mut free_shares: Shares = member.all_shares - member.reserved_shares;
            let mut still_voter = false;
            if let Some(bond_remains) = member.voter_bond {
                free_shares -= bond_remains;
                still_voter = true;
            }
            ensure!(to_burn <= free_shares, "not enough free shares to burn");
            // TODO: make this burn_ratio more dilutive
            let burn_ratio = Permill::from_rational_approximation(to_burn, Self::total_shares());
            let proportionate_balance = burn_ratio * Self::pot();
            // NOTE: if the above doesn't work, multiply numerator in `from_rational_approximation` by Self::pot()
            let _ = T::Currency::transfer(&Self::account_id(), &arsonist, proportionate_balance)?;
            // check if the member is leaving the DAO
            if (member.all_shares, member.reserved_shares, still_voter) == (0, 0, false) {
                // member is leaving the DAO
                <Members<T>>::mutate(|members| members.retain(|m| m != &arsonist));
                <MemberInfo<T>>::remove(&arsonist);
            } else {
                // member is staying the DAO but burning some free shares
                member.all_shares -= to_burn;
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

    fn calculate_pbond(
        shares_requested: Shares,
        donation: Option<BalanceOf<T>>,
        direct: bool,
    ) -> Shares {
        // 20 % higher proposal bond base for direct proposals
        let direct_multiplier = Permill::from_percent(20);
        let base_bond: Shares = T::ProposalBond::get();
        let bond = if direct {
            (direct_multiplier * base_bond) + base_bond
        } else {
            base_bond
        };
        if let Some(donated) = donation {
            // expected ratio: 1 share = 1 balance 1-to-1
            let sr: BalanceOf<T> = shares_requested.into();
            let positive = sr >= donated;
            if positive {
                // donation is low relative to shares => higher required proposal bond
                let diff = sr - donated;
                let diff_abs = Permill::from_rational_approximation(diff, donated);
                let delta = diff_abs * bond;
                // (1 + diff_abs)(T::ProposalBond::get())
                let pricy_proposal_bond = delta + bond;
                return pricy_proposal_bond;
            } else {
                // donation is high relative to shares => lower required proposal bond
                let diff = donated - sr;
                let diff_abs = Permill::from_rational_approximation(diff, donated);
                let delta = diff_abs * bond;
                // (1 - diff_abs)(T::ProposalBond::get())
                let cheap_proposal_bond = bond - delta;
                return cheap_proposal_bond;
            }
        }
        // no donation `=>` pure grant
        // TODO: set relatively high proposal bond (could multiply by 2, but too arbitrary)
        bond
    }
    fn calculate_vbond() -> Shares {
        // TODO: increase/decrease based on proposal throughput rolling average
        // (mvp returns constant)
        T::VoteBond::get()
    }
    fn calculate_threshold(shares_requested: Shares, donation: Option<BalanceOf<T>>) -> Shares {
        // TODO: might also rely on
        // -- sponsored vs direct proposal (to prioritize sponsored proposals if necessary)
        // -- proposal throughput targets vs reality

        // temporary hard-coded threshold (> 20% of outstanding shares)
        let total_shares = Self::total_shares();
        let threshold = (total_shares / 5) + 1;
        threshold
    }

    // ----Auxiliary Methods----
    fn reserve_shares(voter: T::AccountId, support: Shares) -> Result {
        let mut member = <MemberInfo<T>>::get(&voter).ok_or("must be a member")?;
        let mut free_shares = member.all_shares - member.reserved_shares;
        if let Some(vbond) = member.voter_bond {
            free_shares -= vbond;
        }
        ensure!(
            support <= free_shares,
            "not enough free shares to signal support"
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

    /// Make proposals (of the spend variety)
    ///
    /// Called by `sponsor` and `propose`
    fn do_propose(
        sponsor: T::AccountId,
        applicant: Option<T::AccountId>,
        support: Shares,
        donation: Option<BalanceOf<T>>,
        shares_requested: Shares,
    ) -> Result {
        let mut sponsor_bond: Shares = 0;
        if applicant.clone().is_some() {
            // sponsored proposal
            sponsor_bond += Self::calculate_pbond(shares_requested, donation, false);
        } else {
            // direct proposal
            sponsor_bond += Self::calculate_pbond(shares_requested, donation, true);
        }

        // check support >= proposal bond, bond the difference if support < proposal_bond
        let lower_support = support < sponsor_bond;
        let mut sponsor_vote: Shares = 0;
        if lower_support {
            // sponsor_bond > support so reserve sponsor_bond
            Self::reserve_shares(sponsor.clone(), sponsor_bond)?;
            sponsor_vote = sponsor_bond;
        } else {
            // sponsor_bond <= support so reserve support
            Self::reserve_shares(sponsor.clone(), support)?;
            sponsor_vote = support;
        }

        let threshold = Self::calculate_threshold(shares_requested, donation);

        // start the voting period
        let vote_start = <system::Module<T>>::block_number();
        let grace_end = None;

        // ayes and nays vectors
        let mut ayes = Vec::new();
        let mut nays: Vec<(T::AccountId, Shares)> = Vec::new();
        ayes.push((sponsor.clone(), sponsor_vote));
        let state: (Shares, Shares) = (sponsor_vote, 0);

        // clone proposer for event emission after proposal insertion
        let s = sponsor.clone();
        let proposal = Proposal {
            sponsor,
            sponsor_vote,
            applicant,
            donation,
            shares_requested,
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
        <Proposals<T>>::insert(hash, proposal);
        Self::deposit_event(RawEvent::Proposed(s, hash, shares_requested, vote_start));
        Ok(())
    }

    /// Voting
    ///
    /// Called by `vote` and `proxy_vote`
    fn do_vote(voter: T::AccountId, hash: T::Hash, support: Shares, approve: bool) -> Result {
        // verify proposal existence
        let mut proposal = Self::proposals(&hash).ok_or("proposal doesnt exist")?;
        ensure!(&proposal.sponsor != &voter, "the sponsor may not vote");

        // check if the proposal has already passed and entered the grace period
        ensure!(
            proposal.grace_end.is_none(),
            "Proposal passed, grace period already started"
        );
        // otherwise, get the current time to check that it is within the voting window
        let now = <system::Module<T>>::block_number();
        if proposal.vote_start + T::VoteWindow::get() < now {
            <StaleProposals<T>>::mutate(|proposals| proposals.push(hash.clone())); // TODO: update with `append` once 3071 merged
            return Err("Proposal is stale, no more voting!");
        }

        let position_yes = proposal.ayes.iter().position(|a| &a.0 == &voter);
        let position_no = proposal.nays.iter().position(|a| &a.0 == &voter);

        if approve {
            if position_yes.is_none() {
                proposal.ayes.push((voter.clone(), support));
                proposal.state.0 += support;
            } else {
                return Err("duplicate vote");
            }
            // executes if the previous vote was no
            if let Some(pos) = position_no {
                // ability to change vote at no cost prevents bribery attacks
                proposal.nays.swap_remove(pos);
                proposal.state.0 += support;
            }
        } else {
            if position_no.is_none() {
                proposal.nays.push((voter.clone(), support));
                proposal.state.1 += support;
            } else {
                return Err("duplicate vote");
            }
            if let Some(pos) = position_yes {
                proposal.ayes.swap_remove(pos);
                proposal.state.1 += support;
            }
        }
        // update MemberInfo to reflect vote reservations
        Self::reserve_shares(voter.clone(), support)?;
        // unnecessary type aliasing
        let total_support = proposal.state.0;
        let total_against = proposal.state.1;
        // TODO: election.threshold should be updated if surrounding state has changed?
        Self::deposit_event(RawEvent::Voted(
            voter,
            now,
            hash,
            approve,
            support,
            total_support,
            total_against,
        ));

        if total_support > proposal.threshold {
            <Passed<T>>::mutate(|pass| pass.push(hash)); // TODO: update with `append` once 3071 merged
            proposal.grace_end = Some(now + T::GraceWindow::get());
            // pass a slice of AccountIds for dissenting voters
            let _ = Self::liberate(&proposal.nays[..]);
        }

        // TODO: add path for when total_against > some_threshold
        // --> slash some proposal bonds in this event? some negative incentive?
        Ok(())
    }

    /// Execution
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    /// TODO: create more ensure statements and prevent if statement heLL
    fn issuance(n: T::BlockNumber) -> Result {
        <Passed<T>>::get().into_iter().for_each(|hash| {
            // if the proposal exists,
            if let Some(p) = Self::proposals(hash) {
                // if after the grace period ends (wait until then to execute)
                if let Some(end) = p.grace_end {
                    if end <= n {
                        // check if anyone donated
                        if let Some(donate) = p.donation {
                            if let Some(applied) = p.applicant.clone() {
                                // transfer donation from applicant
                                T::Currency::unreserve(&applied, donate);
                                T::Currency::transfer(&applied, &Self::account_id(), donate);
                            } else {
                                // transfer donation from sponsor (direct proposal)
                                T::Currency::unreserve(&p.sponsor, donate);
                                T::Currency::transfer(&p.sponsor, &Self::account_id(), donate);
                            }
                        }

                        // handle both cases of sponsored and/or direct proposal
                        if let Some(apper) = p.applicant {
                            // not a direct proposal, applicant is outside non-member
                            let all_shares = p.shares_requested;
                            let reserved_shares: Shares = 0;
                            let voter_bond = None;
                            <MemberInfo<T>>::insert(
                                &apper,
                                Member {
                                    all_shares,
                                    reserved_shares,
                                    voter_bond,
                                },
                            );
                            <Members<T>>::mutate(|mems| mems.push(apper.clone()));
                        // `append` with 3071
                        } else {
                            // direct proposal, sponsor information changed
                            <MemberInfo<T>>::mutate(&p.sponsor, |mem| {
                                if let Some(memb) = mem {
                                    memb.all_shares += p.shares_requested;
                                }
                            });
                        }

                        // update the shares requested
                        let shares_requested = <SharesRequested>::get() - p.shares_requested;
                        <SharesRequested>::put(shares_requested);
                        let total_shares = <TotalShares>::get() + p.shares_requested; // TODO: checked_add
                        <TotalShares>::put(total_shares);

                        // proposer/sponsor is unbonded here
                        Self::liberate(&p.ayes[..]);
                    }
                }
            }
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
            if let Some(proposal) = Self::proposals(&h) {
                let votes: Vec<(T::AccountId, Shares)> =
                    proposal.ayes.into_iter().chain(proposal.nays).collect();
                // proposal/sponsor bond returned below
                Self::liberate(&votes[..]);

                let shares_requested = <SharesRequested>::get() - proposal.shares_requested;
                <SharesRequested>::put(shares_requested);
            }
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
