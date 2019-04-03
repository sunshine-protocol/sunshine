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

// for system imports use syntax like...
// et hash_of_zero = <T as system::Trait>::Hashing::hash_of(&0);
pub trait Trait: balances::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	// can you make transfers and reserve balances with just the `Currency` type above?
	// if so, take next type out
	type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	// should this be `<..<Self as balance::Trait>::Event>;` ?
}

/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance, Hash, BlockNumber: Parameter> {
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
		Proposed(T::Hash, T::AccountId, T::AccountId),
		Aborted(T::Hash, T::AccountId, T::AccountId), // (index, proposer, applicant), but invoked by proposer
		Voted(T::Hash, bool, u32, u32),
		// true if the proposal was processed successfully
		Processed(T::Hash, bool);
		Withdrawal(AccountId, BalanceOf), // successful "ragequit"
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: BalanceOf) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

			// check that too many shares aren't requsted ( 100% is a high upper bound)
			ensure!(shares <= Self::total_shares(), "too many shares requested");

			// check that applicant doesn't have a pending application
			ensure!(!<Applicants>::exists(&applicant), "applicant has pending application");

			// reserve member's bond for proposal
			T::Currency::reserve(&who, Self::proposal_bond())
				.map_err(|_| "proposer's balance too low")?;

			// reserve applicant's tokenTribute for proposal
			T::Currency::reserve(&applicant, tokenTribute)
				.map_err(|_| "applicant's balance too low")?;

			let startTime = <system::Module<T>>::block_number();

			//add applicant
			<Applicants<T>::insert(&applicant, hash);

			let yesVotes = <MemberShares<T>>:get(&who);
			let noVotes = 0u32;
			let maxVotes = <TotalShares<T>::get();
			<TotalSharesRequested<T>>::get() += shares; // check syntax

			let proposal = Proposal {
				who.clone(), 
				applicant, 
				shares, 
				startTime, 
				None, 
				yesVotes, 
				noVotes, 
				maxVotes, 
				false, 
				false, 
				tokenTribute,
			};

			// add proposal hash
			// CHECK proper encoding syntax using Parity-Codec
			let hash = T::Hashing::hash_of(proposal.encode());
			// check the uniqueness of this hash
			ensure!(!<Proposals<T>>::exists(hash), "This uid is already taken, try again");

			<Proposals<T>>::insert(hash, proposal);

			// add yes vote from member who sponsored proposal (and initiate the voting)
			<VotersFor<T>>::mutate(hash, |voters| voters.push(who.clone()));
			// supporting map for `remove_proposal`
			<ProposalsFor<T>>::mutate(who.clone(), |props| props.push(hash));
			// set that this account has voted
			<VoterId<T>>::mutate(hash, |voters| voters.push(who.clone()));
			// set this for maintainability of other functions
			<VoteOf<T>>::insert(&(hash, who), true);

			Self::deposit_event(RawEvent::Proposed(hash));
			Self::deposit_event(RawEvent::Voted(hash, true, yesVotes, noVotes));
		}
		
		/// Allow revocation of the proposal without penalty within the abortWindow
		fn abort(origin, hash: Hash) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");

			// check if proposal exists
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			let proposal = <Proposals<T>>::get(hash);

			// check that the abort is within the window
			ensure!(proposal.startTime + <Molochameleon<T>::AbortWindow() >= <system::Module<T>>::block_number(), "it is past the abort window");

			// return the proposalBond back to the proposer because they aborted
			T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
			// and the applicant's tokenTribute to the applicant
			T::Currency::unreserve(&proposal.applicant,
			proposal.tokenTribute);

			proposal.aborted = true;

			Self::remove_proposal(hash)?;

			Self::deposit_event(RawEvent::Aborted(hash, proposal.proposer, proposal.applicant));

			Ok(())
		}

		fn vote(origin, hash: Hash, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Module Module");
			
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			// load proposal
			let proposal = <Proposals<T>>::get(hash);

			ensure!((proposal.startTime + <Molochameleon<T>>::voting_period() >= <system::Module<T>>::block_number()) && !proposal.passed, "The voting period has passed with yes: {} no: {}", proposal.yesVotes, proposal.noVotes);

			// check that member has not yet voted
			ensure(!<VoteOf<T>>::exists(hash, who.clone()), "voter has already submitted a vote on this proposal");

			// FIX unncessary conditional path
			if approve {
				<VotersFor<T>>::mutate(hash, |voters| voters.push(who.clone()));
				<ProposalsFor<T>>::insert(who.clone(), |props| props.push(hash));
				<VoterId<T>>::mutate(hash, |voters| voters.push(who.clone()));
				<VoteOf<T>>::insert(&(hash, who), true);

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

			Self::deposit_event(RawEvent::Voted(hash, approve, proposal.yesVotes, proposal.noVotes));
		}

		fn process(hash: Hash) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			let proposal = <Proposals<T>>::get(hash);

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

					Self::remove_proposal(hash);

					let late_time = <system::Module<T>>::block_number - proposal.graceStart + <GracePeriod<T>>::get();

					Self::deposit_event(RawEvent::RemoveStale(hash, late_time));
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
			
			Self::remove_proposal(hash); // clean up
		
			Self::deposit_event(RawEvent::Processed(hash, status));

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
		Proposals get(proposals): map Hash => Option<Proposal<T::AccountId, BalanceOf<T>>>;
		// Active Applicants (to prevent multiple applications at once)
		Applicants get(applicants): map T::AccountId => Option<Hash>; // may need to change to &T::AccountId

		/// VOTING
		// map: proposalHash => Voters that have voted (prevent duplicate votes from the same member)
		VoterId get(voter_id): map Hash => Vec<AccountId>;
		// map: proposalHash => yesVoters (these voters are locked in from ragequitting during the grace period)
		VotersFor get(voters_for): map Hash => Vec<AccountId>;
		// inverse of the function above for `remove_proposal` function
		ProposalsFor get(proposals_for): map AccountId => Vec<Hash>;
		// get the vote of a specific voter (simplify testing for existence of vote via `VoteOf::exists`)
		VoteOf get(vote_of): map (Hash, AccountId) => bool;

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

	// Clean up storage maps involving proposals
	fn remove_proposal(hash: Hash) -> Result {
		ensure!(<Proposals<T>>::exists(hash), "the given proposal does not exist");
		let proposal = <Proposals<T>>::get(hash);
		<Proposals<T>>::remove(hash);
		<Applicants<T>>::remove(proposal.applicant);

		let voters = <VotersFor<T>>::get(hash).iter().map(|voter| {
			<ProposalsFor<T>>::mutate(&voter, |hashes| hashes.iter().filter(|hush| hush != hash).collect());
			voter
		});

		<VoterId<T>>::remove(hash);
		<VoterFor<T>>::remove(hash);
		// reduce outstanding share request amount
		<TotalSharesRequested<T>>::set(|total| total -= proposal.shares);

		Ok(())
	} 
}