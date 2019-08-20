/// Sunshine MVD: (2) Minimize Runtime Storage Size
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

// ----------------------------------START TYPES-------------------------------

// native governance token
type Shares = u32;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

// id f
const DAO_ID: ModuleId = ModuleId(*b"py/daofi");

/// Generic enum for representing proposal state
///
/// -- the type indicates when the state will end
pub enum State<T> {
    // application period has commenced
    Application(T),
    // voting period has commenced
    Proposal(T),
    // the grace period has commenced
    Law(T),
}

/// Generic Moloch Proposal
///
/// request `mint_request` Shares in exchange for `donation` balance staked
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, BlockNumber> {
    // application/proposal/law state
    state: State<BlockNumber>,
    // the proposal sponsor must be a member
    sponsor: Option<AccountId>,
    // the sponsor's share bond (also vote in support by default)
    sponsor_bond: Option<Shares>,
    // the applicant (None if direct proposal by existing member)
    applicant: Option<AccountId>,
    // donation to the DAO (None if grant proposal)
    donation: Option<Balance>,
    // mint request for membership shares (None if pure donation)
    mint_request: Option<Shares>,
    // threshold for passage
    threshold: Shares,                      // TODO: abstract into a voting module
    // shares in favor, shares against
    scoreboard: (Shares, Shares),           // TODO: abstract into a voting module
    // supporting voters (voted yes)
    ayes: Vec<(AccountId, Shares)>,
    // against voters (voted no)
    nays: Vec<(AccountId, Shares)>,         // TODO: if enough no votes, then bond slashed `=>` otherwise returned
}

/// Voter state (for lock-in + 'instant' withdrawals)
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Member {
    // total shares owned by the member
    all_shares: Shares,
    // shares reserved based on pending proposal support/voting
    reserved_shares: Shares,
}   // TODO: this structure requires enforcing reserved_shares <= all_shares
    // why is this better than just having `free_shares` and reducing and adding to it?
    // maybe because it allows for atomic edits and eventual consistency wrt all_shares >= reserved_shares

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

    /// Frequency with which stale proposals and applications are purged/cleared/killed
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
        Applicants get(applicants): Vec<T::AccountId>;

        /// Proposals that have been made
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>>>;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        Stale get(stale): Vec<T::Hash>;

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

        fn deposit_event<T>() = default;

        fn apply(origin, mint_request: Option<Shares>, donation: Option<BalanceOf<T>>) -> Result {
            let applicant = ensure_signed(origin)?;
            // if removed, we risk duplicate applications with the same hash?
            ensure!(!Self::is_applicant(&applicant), "only one application per applicant");

            // reserve donation if it exists
            if let Some(donate) = donation {
                T::Currency::reserve(&applicant, donate)
                    .map_err(|_| "applicant can't afford donation")?;
            }

            // could screen here for proposals outside some range?

            let start = <system::Module<T>>::block_number();
            // clone applicant for event emission post proposal insertion
            let a = applicant.clone();
            let prop = Proposal {
                Application(start),
                None,
                None,
                applicant,
                donation,
                mint_request,

            };
            // take hash of application
            let hash = <T as system::Trait>::Hashing::hash_of(&prop);
            // insert application
            <Proposals<T>>::insert(hash, &prop);
            // add applicant to applicant pool
            <Proposals<T>>::mutate(|props| props.push(a.clone())); // TODO: `append` once 3071
            // deposit event
            Self::deposit_event(RawEvent::Applied(a, hash, donation, mint_request);
            Ok(())
        }

        fn sponsor(origin, app_hash: T::Hash, support: Shares) -> Result {
            let sponsor = ensure_signed(origin)?;
            ensure!(Self::is_member(&sponsor), "sponsor must be a member");

            let app = Self::proposals(&app_hash).ok_or("application must exist")?;
            let now = <system::module<T>::block_number();
            // if the sponsor is the applicant, there are required restrictions on voting for self (see `fn propose`)
            match app.state {
                Application(n) => {
                    if n + T::ApplicationWindow::get() < now {
                        <StaleApps<T>>::mutate(|s| s.push(app_hash.clone())); // append after 3071
                        return Err("The window has passed for this application");
                    }
                    // make the proposal
                    Self::do_propose(sponsor, Some(app.applicant), support, app.donation, app.mint_request)?;
                }
                // eventually, could include voters in capital formation to bias down required share threshold
                // could create a market for backing laws that represents the expectation that it will be overturned or changed
                // -- the hard part is creating both sides of such a market...
                _ => return Err("The proposal's state is not of the form `Application(T)`");
            };

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
            if let Some(app) = Self::proposals(&hash) {
                // INVARIANT: an application must only exist iff there are no associated proposals or elections
                // -- this prevents unreserving the donation amount twice inadvertently
                match app.state {
                    Application(n) => {
                        ensure!(app.applicant.is_some(), "only the applicant can abort the proposal");
                        ensure!(&abortee == &app.applicant.expect("the check directly above keeps this unwrap safe qed"), "only the applicant can abort the proposal");
                        if n + T::ApplicationWindow::get() < now {
                            <StaleApps<T>>::mutate(|s| s.push(hash.clone())); // TODO: `append` after 3071 merged
                            return Err("The window has passed for this application");
                        }
                        // return the application if the window hasnt passed (checked above)
                        if let Some(donate) = &app.donation {
                            T::Currency::unreserve(&app.applicant, donate);
                            // TODO: could calculate and charge an abort fee here
                        }
                        <Proposals<T>>::remove(&hash);
                        Self::deposit_event(RawEvent::Aborted(abortee.clone(), hash.clone()));
                        return Ok(());
                    };
                    _ => return Err("can't abort if not in the application state");
                }
            }
            Err("the application was not in the proposal mapping")
        }

        // register as a new voter
        fn register(origin) -> Result {
            let new_voter = ensure_signed(origin)?;

            let mut member = <MemberInfo<T>>::get(&new_voter).ok_or("no info on member!")?;
            ensure!(!Self::is_voter(&new_voter), "must not be an active voter yet");

            // TODO: governance of the formula behind this bond should be more nuanced
            // voter bond should be based on how much voter bonds are already locked up relative to 
            // proposal throughput
            let vote_bond: Shares = Self::calculate_vbond();
            ensure!(member.all_shares - member.reserved_shares > vote_bond, "not enough funds to become a voter at this time");

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
            // need a stateful way to track the voter bond per voter so it can be properlu returned
            // during deregistration (basically, add it back to the struct with Option<Shares>)
            let voter_bond: Shares = Self::calculate_vbond();
            member.reserved_shares -= voter_bond;

            // take the voter out of the associated storage item (prevent future voting)
            <Voters<T>>::mutate(|voters| voters.retain(|v| v != &old_voter));
            let end = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::DeRegisterVoter(old_voter, voter_bond, end));
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
            ensure!(to_burn <= free_shares, "not enough free shares to burn");
            // TODO: make this burn_ratio more dilutive
            let burn_ratio = Permill::from_rational_approximation(to_burn, Self::total_shares());
            let proportionate_balance = burn_ratio * Self::pot();
            // NOTE: if the above doesn't work, multiply numerator in `from_rational_approximation` by Self::pot()
            let _ = T::Currency::transfer(&Self::account_id(), &arsonist, proportionate_balance)?;
            // check if the member is leaving the DAO
            member.all_shares -= to_burn;
            if member.all_shares == 0 {
                // member is leaving the DAO
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

    fn calculate_pbond(
        shares_requested: Shares,
        donation: Option<BalanceOf<T>>,
        direct: bool,
    ) -> Shares {
        // 75 % lower proposal bond base for direct proposals
        // members can do more proposals 
        let direct_multiplier = Permill::from_percent(75);
        let base_bond: Shares = T::ProposalBond::get();
        let bond = if direct {
            base_bond - (direct_multiplier * base_bond)
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
        // CONTINUE FROM HERE
        // vary the application window time according to how much higher/lower the support is relative to the calculated sponsor bond
        // the support should buy a longer application window `=>` this is what capital purchases in this context
        donation: Option<BalanceOf<T>>,
        mint_request: Option<Shares>,
    ) -> Result {
        let mut sponsor_bond: Shares = 0;
        if let Some(apple) = applicant.clone() {
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

        // if direct proposal, then election.app_sender = sponsor
        let mut app_sender = sponsor.clone();
        if let Some(new_mem_applied) = applicant.clone() {
            // not a direct proposal `=>` applier is applicant
            app_sender = new_mem_applied;
        }
        // clone proposer for event emission after proposal insertion
        let s = sponsor.clone();
        let proposal = Proposal {
            sponsor,
            sponsor_vote,
            applicant,
            donation,
            shares_requested,
        };
        let election = Election {
            app_sender, // might require a clone
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
        <Elections<T>>::insert(hash, election);
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
            &election.app_sender != &voter,
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
            support,
            total_support,
            total_against,
        ));

        if total_support > election.threshold {
            <Passed<T>>::mutate(|pass| pass.push(hash)); // TODO: update with `append` once 3071 merged
            election.grace_end = Some(now + T::GraceWindow::get());
            // pass a slice of AccountIds for dissenting voters
            let _ = Self::liberate(&election.nays[..]);
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
            if let Some(e) = Self::elections(hash) {
                // liberal grace period (sometimes longer than necessary)
                if let Some(end) = e.grace_end {
                    if end <= n {
                        if let Some(p) = Self::proposals(hash) {
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
                            if let Some(appled) = p.applicant {
                                // not a direct proposal, applicant is outside non-member
                                let all_shares = p.shares_requested;
                                let reserved_shares: Shares = 0;
                                let voter_bond = None;
                                <MemberInfo<T>>::insert(
                                    &appled,
                                    Member {
                                        all_shares,
                                        reserved_shares,
                                        voter_bond,
                                    },
                                );
                                <Members<T>>::mutate(|mems| mems.push(appled.clone())); // `append` with 3071
                            } else {
                                // direct proposal, sponsor information changed
                                <MemberInfo<T>>::mutate(&p.sponsor, |mem| {
                                    if let Some(memb) = mem {
                                        memb.all_shares += p.shares_requested;
                                    }
                                });
                            }
                            let shares_requested = <SharesRequested>::get() - p.shares_requested;
                            <SharesRequested>::put(shares_requested);
                            let total_shares = <TotalShares>::get() + p.shares_requested; // TODO: checked_add
                            <TotalShares>::put(total_shares);
                        }
                        // proposer/sponsor is unbonded here
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
                // proposal/sponsor bond returned below
                Self::liberate(&votes[..]);
            }
            // TODO: once again, proposal is only called to change SharesRequested
            // -- just add it to the Election struct and save two calls!
            if let Some(proposal) = Self::proposals(&h) {
                let shares_requested = <SharesRequested>::get() - proposal.shares_requested;
                <SharesRequested>::put(shares_requested);
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
