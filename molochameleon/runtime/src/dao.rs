// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use runtime_primitives::traits::{Zero, As, Bounded}; // redefined Hash type below to make Rust compiler happy
use parity_codec::{Encode, Decode};
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap};
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency}; 					// left out LockableCurrency, OnUnbalanced, WithdrawReason, LockIdentifier
use support::dispatch::Result;
use system::ensure_signed;
use rstd::ops::{Mul, Div}; 							// Add, Rem 
use rstd::fmt::Error; 								// The Error type (discover better error handling in the context of Substrate)
use serde_derive::{Serialize, Deserialize};

/// just getting the compiler to work with me on these types (Hash, AccountId)
type Hash = primitives::H256;
type AccountId = u64;
type BalanceOf<T> = <<T as democracy::Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

pub trait Trait: system::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	// overarching event type
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, BalanceOf, Hash, BlockNumber: Parameter> {
	proposer: AccountId,			 // proposer AccountId
	applicant: AccountId,			 // applicant AccountId
	shares: u32, 					 // number of requested shares
	startTime: BlockNumber,			 // when the voting period starts
	graceStart: Option<BlockNumber>, // when the grace period starts (None if not started)
	yesVotes: u32,					 // number of shares that voted yes
	noVotes: u32,					 // number of shares that voted no
	maxVotes: u32,					 // used to check the number of shares necessary to pass
	processed: bool,				 // if processed, true
	passed: bool,					 // if passed, true
	tokenTribute: BalanceOf, 	 	 // tokenTribute; optional (set to 0 if no tokenTribute)
}

// Wrapper around the central pool which is owned by no one, but has a permissioned withdrawal function
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
struct Pool<AccountId> {
	account: AccountId,
}

decl_event!(
	pub enum Event<T> 
	where
		<T as system::Trait>::AccountId 
	{
		Proposed(Hash, AccountId, AccountId),	// (proposal, proposer, applicant)
		Aborted(Hash, AccountId, AccountId),	// (proposal, proposer, applicant)
		Voted(Hash, bool, u32, u32),		// (proposal, vote, yesVotes, noVotes)
		Processed(Hash, bool),		// true if the proposal was processed successfully
		Withdrawal(AccountId, u64),		// => successful "ragequit" (AccountId, Balances)
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: BalanceOf) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");

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

			let yesVotes = <MemberShares<T>>::get(&who);
			let noVotes = 0u32;
			let maxVotes = Self::total_shares();
			Self::total_shares.set(maxVotes + shares);
			let future_state = false;			// overcome compiler error: expected identifier, found keyword `false`

			let proposal = Proposal {
				who, 
				applicant, 
				shares, 
				startTime, 
				None, 
				yesVotes, 
				noVotes, 
				maxVotes, 
				future_state, 
				future_state, 
				tokenTribute,
			};

			// add proposal hash
			let hash = T::Hashing::hash_of(proposal.encode()); // CHECK proper encoding syntax using Parity-Codec
			// verify uniqueness of this hash
			ensure!(!<Proposals<T>>::exists(hash), "Hash collision X(");

			<Proposals<T>>::insert(hash, proposal);
			//add applicant
			<Applicants<T>>::insert(&applicant, hash);

			// add yes vote from member who sponsored proposal (and initiate the voting)
			<VotersFor<T>>::mutate(hash, |voters| voters.push(&who));
			// supporting map for `remove_proposal`
			<ProposalsFor<T>>::mutate(&who, |props| props.push(hash));
			// set that this account has voted
			<VoterId<T>>::mutate(hash, |voters| voters.push(&who));
			// set this for maintainability of other functions
			<VoteOf<T>>::insert(&(hash, who), true);

			Self::deposit_event(RawEvent::Proposed(hash));
			Self::deposit_event(RawEvent::Voted(hash, true, yesVotes, noVotes));

			Ok(())
		}
		
		/// Allow revocation of the proposal without penalty within the abortWindow
		fn abort(origin, hash: Hash) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");

			// check if proposal exists
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			let proposal = <Proposals<T>>::get(hash);

			// check that the abort is within the window
			ensure!(
				proposal.startTime + Self::abort_window() >= <system::Module<T>>::block_number(),
				"it is past the abort window"
			);

			// return the proposalBond back to the proposer because they aborted
			T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
			// and the tokenTribute to the applicant
			T::Currency::unreserve(&proposal.applicant,
			proposal.tokenTribute);

			proposal.aborted = true;

			Self::remove_proposal(hash)?;

			Self::deposit_event(RawEvent::Aborted(hash, proposal.proposer, proposal.applicant));

			Ok(())
		}

		fn vote(origin, hash: Hash, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");
			
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			// load proposal
			let proposal = <Proposals<T>>::get(hash);

			ensure!(
				(proposal.startTime + <VotingPeriod<T>>::get() >= <system::Module<T>>::block_number()) 
				&& !proposal.passed, 
				format!("The voting period has passed with yes: {} no: {}", proposal.yesVotes, proposal.noVotes)
			);

			// check that member has not yet voted
			ensure!(<VoteOf<T>>::exists(hash, &who), "voter has already submitted a vote on this proposal");

			// FIX unncessary conditional path
			if approve {
				<VotersFor<T>>::mutate(hash, |voters| voters.push(&who));
				<ProposalsFor<T>>::insert(&who, |props| props.push(hash));
				<VoterId<T>>::mutate(hash, |voters| voters.push(&who));
				<VoteOf<T>>::insert(&(hash, who), true);

				// to bound dilution for yes votes
				if <TotalShares<T>>::get() > proposal.maxVotes {
					proposal.maxVotes = <TotalShares<T>>::get();
				}
				proposal.yesVotes += Self::member_shares(&who);

			} else {
				proposal.noVotes += Self::member_shares(&who);
			}

			// proposal passes => switch to the grace period (during which nonsupporters (who have no pending proposals) can exit)
			if proposal.majority_passed() {
				proposal.graceStart = <system::Module<T>>::block_number();
				proposal.passed = true;
			}

			Self::deposit_event(RawEvent::Voted(hash, approve, proposal.yesVotes, proposal.noVotes));

			Ok(())
		}

		fn process(origin, hash: Hash) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");
			ensure!(<Proposals<T>>::exists(hash), "proposal does not exist");

			let proposal = <Proposals<T>>::get(hash);

			// if dilution bound not satisfied, wait until there are more shares before passing
			ensure!(
				<TotalShares<T>>::get().checked_mul(<DilutionBound<T>>::get()) > proposal.maxVotes, 
				"Dilution bound not satisfied; wait until more shares to pass vote"
			);
			
			let grace_period = (
				(proposal.graceStart <= <system::Module<T>>::block_number()) 
				&& (<system::Module<T>>::block_number() < proposal.graceStart + <GracePeriod<T>>::get())
			);
			let status = proposal.passed;

			if (!grace_period && status) {
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
			} else if (grace_period && status) {
				// transfer the proposalBond back to the proposer because they aborted
				T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
				// and the applicant's tokenTribute
				T::Currency::unreserve(&proposal.applicant,
				proposal.tokenTribute);

				// HARDCODED PROCESSING REWARD (todo: make more flexible)
				// transaction fee for proposer and processer comes from tokenTribute
				let txfee = proposal.tokenTribute * 0.05; // check if this works (underflow risk?)
				<BalanceOf<T>>::make_transfer(&proposal.applicant, &who, txfee);
				<BalanceOf<T>>::make_transfer(&proposal.proposer, &who, txfee);

				let netTribute = proposal.tokenTribute * 0.9;

				// transfer tokenTribute to Pool
				let poolAddr = Self::pool_address();
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
			} else {
				Err("The proposal did not pass")
			}
			
			Self::remove_proposal(hash);
		
			Self::deposit_event(RawEvent::Processed(hash, status));

			Ok(())
		}

		fn rage_quit(origin, sharesToBurn: u32) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");

			let shares = <MemberShares<T>>::get(&who);
			ensure!(shares >= sharesToBurn, "insufficient shares");

			// check that all proposals have passed
			ensure!(<ProposalsFor<T>>::get(&who).iter()
					.all(|prop| prop.passed && 
						(<system::Module<T>>::block_number() <= prop.graceStart + Self::grace_period()),
						"All proposals have not passed or exited the grace period"
					)
			);

			Self::pool_address().withdraw(&who, sharesToBurn);

			Ok(())
		}
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Dao {
		// relevant for parameterization of voting periods
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // TODO parameterize 7 days
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // ""  
		AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(1); // "" 1 day
		ProposalBond get(proposal_bond) config(): u32;		// could make this T::Balance
		DilutionBound get(dilution_bound) config(): u32;

		/// TRACKING PROPOSALS
		// Proposals that have been made (equivalent to `ProposalQueue`)
		Proposals get(proposals): map Hash => Proposal<T::AccountId, BalanceOf, T::Hash, T::BlockNumber>;
		// Active Applicants (to prevent multiple applications at once)
		Applicants get(applicants): map T::AccountId => Option<Hash>; // may need to change to &T::AccountId

		/// VOTING
		// map: proposalHash => Voters that have voted (prevent duplicate votes from the same member)
		VoterId get(voter_id): map Hash => Vec<T::AccountId>;
		// map: proposalHash => yesVoters (these voters are locked in from ragequitting during the grace period)
		VotersFor get(voters_for): map Hash => Vec<T::AccountId>;
		// inverse of the function above for `remove_proposal` function
		ProposalsFor get(proposals_for): map T::AccountId => Vec<Hash>;
		// get the vote of a specific voter (simplify testing for existence of vote via `VoteOf::exists`)
		VoteOf get(vote_of): map (Hash, T::AccountId) => bool;

		/// Dao MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current Dao members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current Dao members

		/// INTERNAL ACCOUNTING
		// Address for the pool
		PoolAdress get(pool_address) config(): Pool<T::AccountId>;
		// Number of shares across all members
		TotalShares get(total_shares) config(): u32; 
		// total shares that have been requested in unprocessed proposals
		TotalSharesRequested get(total_shares_requested): u32; 
	}
}

impl<AccountId, BalanceOf, Hash, BlockNumber> Proposal<AccountId, BalanceOf, Hash, BlockNumber> {
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

impl<AccountId> Pool<AccountId> {
	pub fn withdraw(&self, receiver: AccountId, sharedBurned: u32) -> Result { 
		// Checks on identity made in `rage_quit`, the only place in which this is called

		// CHECK: Can we do these calculations w/o the `BalanceOf` type with just `Currency<T>`? If so, how?
		let amount = BalanceOf(&self.account).mul(sharedBurned).div(Self::total_shares());
		BalanceOf::make_transfer(&self.account, &receiver, amount)?;

		Self::deposit_event(RawEvent::Withdrawal(receiver, amount));

		Ok(())
	}
}

impl<T: Trait> Module<T> {
	pub fn is_member(who: &T::AccountId) -> Result {
		Self::active_members().iter()
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
		<VotersFor<T>>::remove(hash);
		// reduce outstanding share request amount
		<TotalSharesRequested<T>>::set(|total| total -= proposal.shares);

		Ok(())
	} 
}