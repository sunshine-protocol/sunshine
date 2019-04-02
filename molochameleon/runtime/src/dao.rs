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

// CODE TODO (Overall)
// add Results for better error propagation (see bank function for example)
// ---> pretty much for every function which returns a bool
// ---> any function that uses `?` to propagate errors
// ---> add appropriate error, Ok(()) bounds (look at Rust book and examples)
// rethink when things should be u32 vs `balance` vs `BalanceOf` 
// ---> I don't want to import Balance so I should just work with the Currency trait if possible (minimize dependencies)
// `cargo clippy` this shit
// consider following the style guide (less than 120 characters per line, etc)

pub trait Trait: system::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	// can you make transfers and reserve balances with just the `Currency` type above?
	// if so, take this out
	type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// To implement a PriorityQueue with the map <Proposals<T>> kept in `decl_storage`
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
// what are the costs of using a separate enum to manage state transitions {processed, passed, aborted}
// decided against it because seems redundant with stuff in `decl_event`, but they do provide separate functionality

#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
struct Pool<AccountId> {
	account: AccountId,
}

impl Pool {
	pub fn withdraw(&self, receiver: T::AccountId, value: BalanceOf) -> Result {
		// note: necessary checks are done in `rageQuit` function

		// CHECK: can you check origin for this function (does it bridge two calls) TEST
		let who = ensure_signed(origin)?

		<balances::Module<T>>::make_transfer(&who, &receiver, value)?; // correct call?
		// TODO using `From` of `BalanceOf` to cast from u32 (good pattern to be aware of)

		// CHECK: Can we do these calculations with the `BalanceOf` type?
		let amount = BalanceOf(self.account).mul(<MemberShares<T>>::get(receiver).div(<TotalShares<T>>::get()));
		<BalanceOf<T>>::make_transfer(&who, &receiver, amount)?;

		Self::deposit_event(RawEvent::Withdrawal(receiver, value));
	}
}

decl_event!(
	pub enum Event<T> 
	where
		<T as system::Trait>::AccountId 
	{
		Proposed(ProposalIndex, T::AccountId, T::AccountId),
		Aborted(ProposalIndex, T::AccountId, T::AccountId), // (index, proposer, applicant), but invoked by proposer
		Voted(ProposalIndex, bool, u32, u32),
		Withdrawal(AccountId, BalanceOf), // successful "ragequit"
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// config before intiial launch (otherwise look at `Treasury` config function for reconfig functionality (which introduces an attack vector))
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: BalanceOf) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

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
			// set that this account has voted
			<VoterId<T>>::mutate(count, |voters| voters.push(who.clone()));
			// set this for maintainability of other functions
			<VoteOf<T>>::insert(&(count, who), true);
			// protect against rage quitting from proposer
			<HighestYesIndex<T>>::mutate(who.clone(), count);
			
			let yesVotes = <MemberShares<T>>:get(&who);
			let noVotes = 0u32;
			let maxVotes = <TotalShares<T>::get();
			let startTime = <system::Module<T>>::block_number();
			<TotalSharesRequested<T>>::get() += shares; // check syntax

			<Proposals<T>>::insert(count, Proposal { who, applicant, shares, startTime, yesVotes, noVotes, maxVotes, false, false, false, tokenTribute });

			Self::deposit_event(RawEvent::Proposed(count));
			Self::deposit_event(Raw_Event::Voted(count, true, yesVotes, noVotes));
		}

		// enable the member who made a proposal to abort
		// think long and hard about timing attacks
		fn abort(origin, index: ProposalIndex) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

			// check if proposal exists
			// consider using the length check that MolochDAO uses
			// ensure!(index < <Proposals<T>>.length()); // not correct syntax
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");

			let proposal = <Proposals<T>>::get(index);
			proposal.aborted = true;

			// check it is within the abort window
			ensure!(proposal.startTime + <Module<T>::AbortWindow() >= <system::Module<T>>::block_number(), "it is past the abort window");

			// check if already aborted
			ensure!(!proposal.aborted, "proposal already aborted");

			// CALL `remove_proposal` function

			Self::deposit_event(Raw_Event::Aborted(index, proposal.proposer, proposal.applicant));
		}

		fn vote(origin, index: ProposalIndex, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");
			
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");
			// ensure!(proposalIndex < <Module<T>>::proposal_count())

			// load proposal using `Proposals: map Index => Option<Proposal<T::AccountId, BalanceOf<T>>;
			let proposal = <Proposals<T>>::get(index);

			// check that it is within the voting period
			ensure!(proposal.startTime + <Module<T>::voting_period() >= <system::Module<T>>::block_number(), "it is past the voting period");

			ensure!(!proposal.aborted, "The proposal has been aborted");

			// check that the member has not yet voted
			ensure(!<VoteOf<T>>::exists(index, who.clone()), "voter has already submitted a vote on this proposal");

			if approve {
				<VotersFor<T>>::mutate(index, |voters| voters.push(who.clone()));
				<VoterId<T>>::mutate(index, |voters| voters.push(who.clone()));
				<VoteOf<T>>::insert(&(index, who), true);

				if index > <HighestYesIndex<T>>::get() {
					<HighestYesIndex<T>>::mutate(who.clone(), index);
				}
				// to bound dilution for yes votes
				if <TotalShares<T>>::get() > proposal.maxVotes {
					proposal.maxVotes = <TotalShares<T>>::get();
				}
				proposal.yesVotes += <MemberShares<T>>:get(&who);

			} else {
				proposal.noVotes += <MemberShares<T>>:get(&who);
			}

			Self::deposit_event(RawEvent::Voted(ProposalIndex, approve, proposal.yesVotes, proposal.noVotes));
		}

		fn process(index: ProposalIndex) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");
			// ensure!(proposalIndex < <Module<T>>::proposal_count());

			let proposal = <Proposals<T>>::get(index);
			
			// TODO: check if the time has passed (must be in grace period)
			ensure!(!proposal.processed, "proposal has already been processed");
			ensure!(index == 0 || <Proposals<T>>::get(index).checked_sub(1).processed, "previous proposal must be processed");

			proposal.processed = true;

			<TotalSharesRequested<T>>::get().checked_sub(proposal.shares); // check correct syntax

			let bool didPass = proposal.majority_passed();

			if <TotalShares<T>>::get().checked_mul(<DilutionBound<T>>::get()) > proposal.maxVotes {
				didPass = false; // proposal fails if dilutionBound is exceeded
			}

			// PASSED
			let punish = false; // changes to true if condition isn't true (proposal doesn't pass)
			if (didPass && !proposal.aborted) {
				proposal.passed = true;

				// if applicant is already a member, add to their existing shares
				if proposal.applicant.is_member() {
					<MemberShares<T>>::mutate(proposal.applicant, |shares| shares += proposal.shares);
				} else {
					// if applicant is a new member, create a new record for them

				}

				// mint new shares
				<TotalShares<T>>::mutate(|total| total += proposal.shares); // check syntax

				// transfer tokenTribute to common Pool

			} else {
				// PROPOSAL FAILED OR ABORTED

				// don't need to return tokens because they were never given...

				punish = true; // to punish the proposer (see below) (just exact a cost)
			}

			// give msg.sender the processing reward (subtracted from proposer bond)

			if punish {
				// transfer the rest of the proposer bond to the POOL
			}

			Self::deposit_event(RawEvent::Processed(index, true)); // consider adding more fields to this event

			// do I need to remove Proposal from the ProposalQueue?
			// any other dependent maps or data structures
		}

		// HOW TO SLASH (from `Treasury`)
		// /// Reject a proposed spend. The original deposit will be slashed.
		// fn reject_proposal(origin, #[compact] proposal_id: ProposalIndex) {
		// 	T::RejectOrigin::ensure_origin(origin)?;
		// 	let proposal = <Proposals<T>>::take(proposal_id).ok_or("No proposal at that index")?;

		// 	let value = proposal.bond;
		// 	let imbalance = T::Currency::slash_reserved(&proposal.proposer, value).0;
		// 	T::ProposalRejection::on_unbalanced(imbalance);
		// }

		fn rage_quit(sharesToBurn: u32) {
				// check signed
				// check they're a member
				// use HighestYesIndex to check if they can ragequit (or if they're locked in)
				// check that they can burn that many shares (have at least that many shares)

				// burn the shares (subtract from member_shares)
				// subtract from total_shares

				// withdraw from Pool 
				// the *correct* amount

				// event -- RageQuit
		}Î
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Module {
		/// CONFIG (like the constructor values)
		PeriodDuration get(period_duration) config(): u32, 			// relevant for parameterization of voting periods
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // convert from block numbers to days (currently just 7 days)
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // ""  
		AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(1); // "" 1 day
		// Amount of funds that must be put at stake (by a member) for making a proposal. (0.1 ETH in MolochModule)
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

		/// Module MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current Module members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current Module members

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
			return (self.yesVotes > self.maxVotes.checked_div(2)) 
		} else { 
			return (self.yesVotes > (self.maxVotes.checked_add(1).checked_div(2)))
		};
	}
}

impl<T: Trait> Module<T> {
	pub fn is_member(who: &T::AccountId) -> bool {
		<Module<T>>::active_members().iter()
			.any(|&(ref a, _)| a == who)?
		
		Ok(());
	}

	// ensure adequate checks are made before this is called (and only specific functions can call it in context)
	// abstracts clean up of core maps involving proposals and applicant
	fn remove_proposal(index: ProposalIndex) -> Result {
		ensure!(<Proposals<T>>::exists(index), "the given proposal does not exist");

		// <Proposals<T>>::remove()
		// Applicants<T>>::remove()
		// <ProposalCount<T>>::set(|count| count -= 1); // fix syntax
		
		// fix HighestYesIndex
		// VoterId
		// VotersFor
		// VoteOf
		// TotalShareRequested -= proposal.share`
	}
}