// These are needed because of the `Proposal` struct ¯\_(ツ)_/¯ 
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use primitives::traits::{Zero, As, Bounded}; // don't really use these (could use Hash)
use parity_codec::{Encode, Decode};
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap}; // don't use all of these...
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency, OnUnbalanced}; // WithdrawReason, LockIdentifier, LockableCurrency
use support::dispatch::Result;
use system::ensure_signed;
use rstd::ops::{Add, Mul, Div, Rem};

// when is this used or necessary???
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
// presumably for slashing the member who proposed something that is rejected?
type NegativeImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

pub trait Trait: system::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	/// Handler for the unbalanced decrease when slashing for a rejected proposal. (from `Treasury`)
	type ProposalRejection: OnUnbalanced<NegativeImbalanceOf<Self>>;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// To emulate a PriorityQueue with a map kept in `decl_storage`
type ProposalIndex = u32;
/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance, BlockNumber: Parameter> {
	proposer: AccountId,			// proposer AccountId
	applicant: AccountId,			// applicant AccountId
	shares: u32, 					// number of requested shares
	startTime: BlockNumber,			// when the voting period starts
	yesVotes: u32,					// number of shares that voted yes
	noVotes: u32,					// number of shares that voted no
	maxVotes: u32,					// used to check the number of shares necessary to pass
	processed: bool,				// if processed, true
	passed: bool,					// if passed, true
	aborted: bool,					// of aborted, true
	tokenTribute: Balance, 			// tokenTribute
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> 
	where
		<T as system::Trait>::AccountId 
	{
		Summoned(T::AccountId),
		/// A new proposal has been submitted 
		Proposed(ProposalIndex),
		// for aborting a proposal while it is being voted on (AccountId is applicant AccountId)
		Aborted(ProposalIndex, AccountId),
		/// A proposal has been voted on by given account (approve, yes_votes, no_votes)
		Voted(ProposalIndex, bool, u32, u32),
		/// A proposal was approved by the required threshold.
		Approved(ProposalIndex),
		/// A proposal was not approved by the required threshold.
		Rejected(ProposalIndex),
		/// The proposal was processed (executed); `bool` is true if returned without error
		Processed(ProposalIndex, bool),
		// The member `ragequit` the DAO
		// TODO: NEED TO PROTECT AGAINST TIMING ATTACKS FOR THIS...
		Ragequit(AccountId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// config before intiial launch (otherwise look at `Treasury` config function for reconfig functionality (which introduces an attack vector))
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Malkam DAO");

			// check that too many shares aren't requsted (< max set in config)
			ensure!(shares <= Self::max_shares(), "too many shares requested");

			// check that applicant doesn't have a pending application
			ensure!(!<Applicants>::exists(&applicant), "applicant has pending application");
			// REMINDER: REMOVE APPLICANT WHEN PROPOSAL PASSES OR IS REJECTED

			// reserve member's bond for proposal
			T::Currency::reserve(&who, Self::proposal_bond())
				.map_err(|_| "proposer's balance too low")?;

			//add applicant
			<Applicants<T>::insert(&applicant, count);
			
			// add proposal (TODO: TEST CORRECT INDEXING HERE)
			let count = Self::proposal_count(); // how does this actually work? Must config correctly!
			<ProposalCount<T>>::put(count + 1);

			// add yes vote from member who sponsored proposal (and initiate the voting)
			<VotersFor<T>>::mutate(count, |voters| voters.push(who.clone()));
			// say that this account has voted
			<VoterId<T>>::mutate(count, |voters| voters.push(who.clone()));
			// set this for maintainability of other functions
			<VoteOf<T>>::insert(&(count, who), true);
			// protect against rage quitting from proposer
			<HighestYesIndex<T>>::mutate(who.clone(), count);
			
			let yesVotes = <MemberShares<T>>:get(&who);
			let noVotes = 0u32;
			let maxVotes = <TotalShares<T>::get();
			let startTime = <system::Module<T>>::block_number();

			<Proposals<T>>::insert(count, Proposal { who, applicant, shares, startTime, yesVotes, noVotes, maxVotes, false, false, false, tokenTribute });

			Self::deposit_event(RawEvent::Proposed(count));
			Self::deposit_event(Raw_Event::Voted(count, true, yesVotes, noVotes));
		}

		// enable the member who made a proposal to abort
		// think long and hard about timing attacks
		fn abort(origin, proposal: ProposalIndex) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Malkam DAO");

			// OPEN QUESTION: is there a cost to aborting?

		}

		fn vote(origin, index: ProposalIndex, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Malkam DAO");
			
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");

			// load proposal using `Proposals: map Index => Option<Proposal<T::AccountId, BalanceOf<T>>;
			let prop = <Proposals<T>>::get(index);

			// check that it is within the voting period
			ensure!(prop.startTime + <Malkam<T>::voting_period() >= <system::Module<T>>::block_number(), "it is past the voting period");

			ensure!(!prop.aborted, "The proposal has been aborted");

			// check that the member has not yet voted
			ensure(!<VoteOf<T>>::exists(index, who.clone()), "voter has already submitted a vote on this proposal");

			if approve {
				<VotersFor<T>>::mutate(index, |voters| voters.push(who.clone()));
				<VoterId<T>>::mutate(index, |voters| voters.push(who.clone()));
				<VoteOf<T>>::insert(&(index, who), true);

				if index > <HighestYesIndex<T>>::get() {
					<HighestYesIndex<T>>::mutate(who.clone(), index);
				}
				// consider setting maximum of total shares encountered at a yes vote - to bound dilution for yes voters
				prop.yesVotes += <MemberShares<T>>:get(&who);

			} else {
				prop.noVotes += <MemberShares<T>>:get(&who);
			}

			Self::deposit_event(RawEvent::Voted(ProposalIndex, approve, prop.yesVotes, prop.noVotes));
		}

		fn process(index: ProposalIndex) {
			
			ensure!()
			// if rejected...
			
			// weight votes by number of shares no?
		}

		/// Reject a proposed spend. The original deposit will be slashed.
		fn reject_proposal(origin, #[compact] proposal_id: ProposalIndex) {
			T::RejectOrigin::ensure_origin(origin)?;
			let proposal = <Proposals<T>>::take(proposal_id).ok_or("No proposal at that index")?;

			let value = proposal.bond;
			let imbalance = T::Currency::slash_reserved(&proposal.proposer, value).0;
			T::ProposalRejection::on_unbalanced(imbalance);
		}

		fn rage_quit() {
		}Î
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Malkam {
		/// CONFIG (like the constructor values)
		PeriodDuration get(period_duration) config(): u32, 			// relevant for parameterization of voting periods
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // convert from block numbers to days (currently just 7 days)
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // ""  
		AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(1); // "" 1 day
		// Amount of funds that must be put at stake (by a member) for making a proposal. (0.1 ETH in MolochDAO)
		ProposalBond get(proposal_bond) config(): u32;		// could make this T::Balance
		DilutionBound get(dilution_bound) config(): u32;
		ProcessingReward get(processing_reward) config(): u32;	// could also make this T::Balance or BalanceOf<T>

		/// TRACKING PROPOSALS
		// Proposals that have been made (equivalent to `ProposalQueue`)
		Proposals get(proposals): map ProposalIndex => Option<Proposal<T::AccountId, BalanceOf<T>>>;
		// Active Applicants (to prevent multiple applications at once)
		Applicants get(applicants): map T::AccountId => Option<ProposalIndex>; // may need to change to &T::AccountId
		// Number of proposals that have been made.
		ProposalCount get(proposal_count): ProposalIndex;

		/// VOTING
		// to protect against rage quitting (only works if the proposals are processed in order...)
		HighestYesIndex get(highest_yes_index): map T::AccountId => Option<ProposalIndex>;
		// map: proposalIndex => Voters that have voted (prevent duplicate votes from the same member)
		VoterId get(voter_id): map ProposalIndex => Vec<AccountId>;
		// map: proposalIndex => yesVoters (these voters are locked in from ragequitting during the grace period)
		VotersFor get(voters_for): map ProposalIndex => Vec<AccountId>;
		// get the vote of a specific voter (simplify testing for existence of vote via `VoteOf::exists`)
		VoteOf get(vote_of): map (ProposalIndex, AccountId) => bool;

		/// DAO MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current DAO members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current DAO members

		/// INTERNAL ACCOUNTING
		// Number of shares across all members
		TotalShares get(total_shares) config(): u32; 
		// total shares that have been requested in unprocessed proposals
		TotalSharesRequested get(total_shares_requested): u32; 
	}
}

/// figure out the correct trait bound for this
impl Proposal {
	// more than half shares voted yes
	pub fn majority_passed(&self) -> bool {
		// do I need the `checked_div` flag?
		if self.maxVotes % 2 == 0 { 
			return (self.yesVotes > self.maxVotes.check_div(2)) 
		} else { 
			return (self.yesVotes > (self.maxVotes.checked_add(1).checked_div(2)))
		};
	}
}

impl<T: Trait> Module<T> {
	pub fn is_member(who: &T::AccountId) -> bool {
		<Malkam<T>>::active_members().iter()
			.any(|&(ref a, _)| a == who)
	}

	pub fn can_quit(who: &T::AccountId) -> bool {
		// intuition is to use `HighestYesIndex`
	}
}