#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use primitives::traits::{Hash, Zero, As, Bounded};
use parity_codec::{Encode, Decode};
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap};
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency, OnUnbalanced, WithdrawReason, LockIdentifier}; // left out LockableCurrency
use support::dispatch::Result;
use system::ensure_signed;
use rstd::ops::{Add, Mul, Div, Rem};

pub trait Trait: system::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	// can you make transfers and reserve balances with just the `Currency` type above?
	// if so, take next type out
	type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// To implement a PriorityQueue with the map <Proposals<T>> kept in `decl_storage`
type ProposalIndex = u32;
/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance, Hash, BlockNumber: Parameter> {
	uid: Hash,						// unique identifier
	// consider adding an IPFS Hash with <Proposal Details>; like git commit number or something could even work
	// or make the uid this number optionally?
	proposer: AccountId,			// proposer AccountId
	applicant: AccountId,			// applicant AccountId
	shares: u32, 					// number of requested shares
	startTime: BlockNumber,			// when the voting period starts
	graceStart: Option<BlockNumber>, // when the grace period starts (None if not started)
	yesVotes: u32,					// number of shares that voted yes
	noVotes: u32,					// number of shares that voted no
	maxVotes: u32,					// used to check the number of shares necessary to pass
	processed: bool,				// if processed, true
	passed: bool,					// if passed, true
	tokenTribute: Option<Balance>, 	// tokenTribute; optional
}

// Wrapper around the central pool which is owned by no one, but has a withdrawal function that follows the protocol
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
struct Pool<AccountId> {
	// take advantage of the Currency traits for optimal implementation
	account: AccountId,
}

impl Pool {
	pub fn withdraw(&self, receiver: T::AccountId, sharedBurned: u32) -> Result { // checks made in `RageQuit` as well

		// CHECK: Can we do these calculations w/o the `BalanceOf` type with just `Currency<T>`? If so, how?
		let amount = BalanceOf(&self.account).mul(sharedBurned).div(<TotalShares<T>>::get());
		<BalanceOf<T>>::make_transfer(&self.account, &receiver, amount)?;

		Self::deposit_event(RawEvent::Withdrawal(receiver, amount));

		Ok(())
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
		// true if the proposal was processed successfully
		Processed(ProposalIndex, bool);
		Withdrawal(AccountId, BalanceOf), // successful "ragequit"
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: BalanceOf) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

			// check that too many shares aren't requsted (< max set in config)
			ensure!(shares <= Self::max_shares(), "too many shares requested");

			// check that applicant doesn't have a pending application
			ensure!(!<Applicants>::exists(&applicant), "applicant has pending application");

			// reserve member's bond for proposal
			T::Currency::reserve(&who, Self::proposal_bond())
				.map_err(|_| "proposer's balance too low")?;

			// reserve applicant's tokenTribute for proposal
			T::Currency::reserve(&applicant, tokenTribute)
				.map_err(|_| "applicant's balance too low")?;

			//add applicant
			<Applicants<T>::insert(&applicant, count);
			
			// add proposal
			let count = Self::proposal_count();
			<ProposalCount<T>>::put(count + 1);

			// add yes vote from member who sponsored proposal (and initiate the voting)
			<VotersFor<T>>::mutate(count, |voters| voters.push(who.clone()));
			// supporting map for `remove_proposal`
			<ProposalsFor<T>>::mutate(who.clone(), |props| props.push(count));
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

			// set graceTime to None because it doesn't start until the vote is processed correctly
			<Proposals<T>>::insert(count, Proposal { who, applicant, shares, startTime, None, yesVotes, noVotes, maxVotes, false, false, tokenTribute });

			Self::deposit_event(RawEvent::Proposed(count));
			Self::deposit_event(Raw_Event::Voted(count, true, yesVotes, noVotes));
		}
		
		/// Allow revocation of the proposal without penalty within the abortWindow
		fn abort(origin, index: ProposalIndex) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

			// check if proposal exists
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");
			ensure!(index < <Molochameleon<T>>::proposal_count(), "proposal does not exist");

			let proposal = <Proposals<T>>::get(index);

			// check that the abort is within the window
			ensure!(proposal.startTime + <Molochameleon<T>::AbortWindow() >= <system::Module<T>>::block_number(), "it is past the abort window");

			// return the proposalBond back to the proposer because they aborted
			T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
			// and the applicant's tokenTribute to the applicant
			T::Currency::unreserve(&proposal.applicant,
			proposal.tokenTribute);

			proposal.aborted = true;

			Self::remove_proposal(index)?;

			Self::deposit_event(RawEvent::Aborted(index, proposal.proposer, proposal.applicant));

			Ok(())
		}

		fn vote(origin, index: ProposalIndex, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");
			
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");
			ensure!(index < <Molochameleon<T>>::proposal_count(), "proposal does not exist");

			// load proposal
			let proposal = <Proposals<T>>::get(index);

			ensure!((proposal.startTime + <Molochameleon<T>>::voting_period() >= <system::Module<T>>::block_number()) && !proposal.passed, "The voting period has passed with yes: {} no: {}", proposal.yesVotes, proposal.noVotes);

			// check that member has not yet voted
			ensure(!<VoteOf<T>>::exists(index, who.clone()), "voter has already submitted a vote on this proposal");

			// FIX unncessary conditional path
			if approve {
				<VotersFor<T>>::mutate(index, |voters| voters.push(who.clone()));
				<ProposalsFor<T>>::insert(who.clone(), |props| props.push(index));
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

			/// IF PROPOSAL PASSES, SWITCH TO GRACE PERIOD
			if proposal.majority_passed() {
				// start the graceStart
				proposal.graceStart = <system::Module<T>>::block_number();

				proposal.passed = true;
			}

			Self::deposit_event(RawEvent::Voted(ProposalIndex, approve, proposal.yesVotes, proposal.noVotes));
		}

		fn process(index: ProposalIndex) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");
			ensure!(<Proposals<T>>::exists(index), "proposal does not exist");
			// maybe redundant, but bound checking with ProposalQ
			ensure!(proposalIndex < <Molochameleon<T>>::proposal_count(), "proposal does not exist");

			let proposal = <Proposals<T>>::get(index);

			// if dilution bound not satisfied, wait until there are more shares before passing
			ensure!(<TotalShares<T>>::get().checked_mul(<DilutionBound<T>>::get()) > proposal.maxVotes, "Dilution bound not satisfied; wait until more shares to pass vote");
			
			let grace_period = (proposal.graceStart <= <system::Module<T>>::block_number() < proposal.graceStart + <GracePeriod<T>>::get());
			let status = proposal.passed;

			match {
				(!grace_period && status) => {
					// transfer the proposalBond back to the proposer
					T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
					// transfer 50% of the proposal bond to the processer
					T::Currency::transfer(&proposal.proposer, &who, Self::proposal_bond().checked_mul(0.5));
					// return the applicant's tokenTribute
					T::Currency::unreserve(&proposal.applicant,
					proposal.tokenTribute);

					Self::remove_proposal(index);

					let late_time = <system::Module<T>>::block_number - proposal.graceStart + <GracePeriod<T>>::get();

					Self::deposit_event(RawEvent::RemoveStale(index, late_time));
				},
				(grace_period && status) => {

					// transfer the proposalBond back to the proposer because they aborted
					T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
					// and the applicant's tokenTribute
					T::Currency::unreserve(&proposal.applicant,
					proposal.tokenTribute);

					//HARDCODED PROCESSING REWARD (make this logic more obvious in docs; could set in config but adds logic for computing fees...added to todo)
					// transaction fee for proposer and processer comes from tokenTribute
					let txfee = proposal.tokenTribute * 0.05; // check if this works (do I need a checked_mul for underflow for u32)
					<BalanceOf<T>>::make_transfer(&proposal.applicant, &who, txfee);
					<BalanceOf<T>>::make_transfer(&proposal.proposer, &who, txfee);

					let netTribute = proposal.tokenTribute * 0.9;

					// transfer tokenTribute to Pool
					let poolAddr = <Molochameleon<T>>::pool_address();
					<BalanceOf<T>>::make_transfer(&proposal.applicant, &poolAddr, netTribute);

					// mint new shares
					<TotalShares<T>>::set(|total| total += proposal.shares);

					// if applicant is already a member, add to their existing shares
					if proposal.applicant.is_member() {
						<MemberShares<T>>::mutate(proposal.applicant, |shares| shares += proposal.shares);
					} else {
						// if applicant is a new member, create a new record for them
						<MemberShares<T>>::insert(proposal.applicant, proposal.shares);
					}
				}
				_ => {
					Err("The proposal did not pass")
				}
			}
			
			Self::remove_proposal(index); // clean up
		
			Self::deposit_event(RawEvent::Processed(index, status));

			Ok(()) // do I need this here
		}

		fn rage_quit(sharesToBurn: u32) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");

			let shares = <MemberShares<T>>::get(&who);
			ensure!(shares >= sharesToBurn, "insufficient shares");

			// check that all proposals have passed
			ensure!(<ProposalsFor<T>>::get(who.clone()).iter().all(|prop| prop.passed && (<system::Module<T>>::block_number() <= prop.graceStart + <Molochameleon<T>>::grace_period()), "All proposals have not passed or exited the grace period");

			<PoolAddress<T>>::get().withdraw(&who, sharesToBurn);

			Ok(())
		}Î
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Module {
		// relevant for parameterization of voting periods
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // convert from block numbers to days (currently just 7 days)
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // ""  
		AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(1); // "" 1 day
		// Amount of funds that must be put at stake (by a member) for making a proposal. (0.1 ETH in MolochModule)
		ProposalBond get(proposal_bond) config(): u32;		// could make this T::Balance
		DilutionBound get(dilution_bound) config(): u32;

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
		// inverse of the function above for `remove_proposal` function
		ProposalsFor get(proposals_for): map AccountId => Vec<ProposalIndex>;
		// get the vote of a specific voter (simplify testing for existence of vote via `VoteOf::exists`)
		VoteOf get(vote_of): map (ProposalIndex, AccountId) => bool;

		/// Module MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current Module members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current Module members
		PoolAddress get(pool_address): config(): Pool<T::AccountId>;

		/// INTERNAL ACCOUNTING
		// Address for the pool
		PoolAdress get(pool_address) config(): Pool<AccountId>;
		// Number of shares across all members
		TotalShares get(total_shares) config(): u32; 
		// total shares that have been requested in unprocessed proposals
		TotalSharesRequested get(total_shares_requested): u32; 
	}
}

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
	pub fn is_member(who: &T::AccountId) -> Result {
		<Module<T>>::active_members().iter()
			.any(|&(ref a, _)| a == who)?;
		Ok(())
	}

	// Clean up of core maps involving proposals and applicant
	fn remove_proposal(index: ProposalIndex) -> Result {
		ensure!(<Proposals<T>>::exists(index), "the given proposal does not exist");

		let proposal = <Proposals<T>>::get(index)?;
		// ORDER IS EXTREMELY IMPORTANT
		<Proposals<T>>::remove(index).ok_or("No proposal at that index")?;
		<Applicants<T>>::remove(proposal.applicant);
		<ProposalCount<T>>::mutate(|count| count -= 1);

		let voters = <VotersFor<T>>::get(index);
		// USE ITERATOR INSTEAD OF FOR LOOP
		for voter in voters {
			// remove index from PropsFor
			<ProposalsFor<T>>::mutate(&voter, |indexs| indexs.iter().filter(|ind| ind != index).collect());
			// set new HighestYesIndex
			<HighestYesIndex<T>>::mutate(&voter, <ProposalsFor<T>>::get(voter).iter().max());
			<VoteOf<T>>::remove(&(index, voter));
		}

		<VoterId<T>>::remove(index);
		<VoterFor<T>>::remove(index);
		// reduce outstanding share request amount
		<TotalSharesRequested<T>>::set(|count| count -= proposal.shares);

		<Applicants<T>>::remove(proposal.applicant);

		Ok(())
	} 
	// could alternatively have a trait called Proposal that implements a custom Drop() with similar logic but this method's logic only makes sense in the context of Module<T>
}