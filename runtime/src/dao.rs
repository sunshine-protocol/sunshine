// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use runtime_io::{with_storage};
use runtime_primitives::{StorageOverlay, ChildrenStorageOverlay}; // traits::{Zero, As, Bounded}
use parity_codec::{Encode, Decode, HasCompact}; // HasCompact
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap, dispatch::Result}; // Parameter, IsSubType, EnumerableStorageMap
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency, LockableCurrency}; 			// left out OnUnbalanced, WithdrawReason, LockIdentifier
use system::ensure_signed;
use serde_derive::{Serialize, Deserialize};

/// type aliasing for compilation
type AccountId = u64;

pub trait Trait: system::Trait {
	// the staking balance
	type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;

	// overarching event type
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// Sometimes I use this and sometimes I just use T::Currency ¯\_(ツ)_/¯
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// Wrapper around AccountId with permissioned withdrawal function (for ragequit)
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Pool<AccountId> {
	// The account for which the total funds are locked
	main: AccountId,
	// // Insurance pool for aligning incentives in a closed system
	// insurance: AccountId,
	// Total Shares Issue
	shares: u32,
}

/// Encoded and used as a UID for each Proposal
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
struct Base<AccountId, Balance> {
	proposer: AccountId,
	applicant: AccountId,
	sharesRequested: u32,
	tokenTribute: Balance,
}

/// A proposal to lock up tokens in exchange for shares
pub struct Proposal<AccountId, Balance, BlockNumber> {
	base_hash: Vec<u8>,				 // hash of the proposal
	proposer: AccountId,			 // proposer AccountId (must be a member)
	applicant: AccountId,			 // applicant AccountId
	shares: u32, 					 // number of requested shares
	startTime: BlockNumber,			 // when the voting period starts
	graceStart: Option<BlockNumber>, // when the grace period starts (None if not started)
	yesVotes: u32,					 // number of shares that voted yes
	noVotes: u32,					 // number of shares that voted no
	maxVotes: u32,					 // used to check the number of shares necessary to pass
	passed: bool,					 // if passed, true
	processed: bool,				 // if processed, true
	tokenTribute: Balance, 	 	 	 // tokenTribute; optional (set to 0 if no tokenTribute)
}

decl_event!(
	pub enum Event<T> where Balance = BalanceOf<T>, <T as system::Trait>::AccountId 
	{
		Proposed(Vec<u8>, Balance, AccountId, AccountId),	// (proposal, tokenTribute, proposer, applicant)
		Aborted(Vec<u8>, Balance, AccountId, AccountId),	// (proposal, proposer, applicant)
		Voted(Vec<u8>, bool, u32, u32),						// (proposal, vote, yesVotes, noVotes)
		RemoveStale(Vec<u8>, u64),							// (hash, however_much_time_it_was_late_by)
		Processed(Vec<u8>, Balance, AccountId, bool),		// (proposal, tokenTribute, NewMember, executed_correctly)
		Withdrawal(AccountId, u32, Balance),				// => successful "ragequit" (member, shares, Balances)
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Dao {
		// The length of a voting period in sessions
		pub VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(500);
		// the time after the voting period starts during which the proposer can abort
		pub AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(200);
		// The length of a grace period in sessions
		pub GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(1000);
		pub ProposalFee get(proposal_fee) config(): BalanceOf<T>; // applicant's bond
		pub ProposalBond get(proposal_bond) config(): BalanceOf<T>; // proposer's bond
		pub DilutionBound get(dilution_bound) config(): u32;

		/// TRACKING PROPOSALS
		// Proposals that have been made (impl of `ProposalQueue`)
		pub Proposals get(proposals): map Vec<u8> => Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber>;
		// Active Applicants (to prevent multiple applications at once)
		pub Applicants get(applicants): map T::AccountId => Vec<u8>; // may need to change to &T::AccountId

		/// VOTING
		// map: proposalHash => Voters that have voted (prevent duplicate votes from the same member)
		pub VoterId get(voter_id): map Vec<u8> => Vec<T::AccountId>;
		// map: proposalHash => voters_who_voted_yes (these voters are locked in from ragequitting during the grace period)
		pub VotersFor get(voters_for): map Vec<u8> => Vec<T::AccountId>;
		// inverse of the function above for `remove_proposal` function
		pub ProposalsFor get(proposals_for): map T::AccountId => Vec<Vec<u8>>;
		// get the vote of a specific voter (simplify testing for existence of vote via `VoteOf::exists`)
		pub VoteOf get(vote_of): map (Vec<u8>, T::AccountId) => bool;

		/// Dao MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		pub MemberCount get(member_count) config(): u32; // the number of current DAO members
		pub ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current Dao members
		pub MemberShares get(member_shares): map T::AccountId => u32; // shares of the current Dao members

		/// INTERNAL ACCOUNTING
		// The DAO Pool
		pub DaoPool get(dao_pool) config(): Pool<AccountId>;
		// Number of shares across all members
		pub TotalShares get(total_shares) config(): u32; 
		// total shares that have been requested in unprocessed proposals
		pub TotalSharesRequested get(total_shares_requested): u32; 
	}
	// Bootstrap from Centralization -> Nudge Towards Decentralized Arc
	add_extra_genesis { // see `mock.rs::ExtBuilder::build` for usage
		config(members): Vec<T::AccountId, u32>; // (accountid, sharesOwned)
		config(applicants): Vec<T::AccountId, u32, BalanceOf<T>>; // (accountId, sharesRequested, tokenTribute)
		config(pool): (T::AccountId, u32);
		// check that \sum{member_shares} == pool.shares
		build(|storage: &mut runtime_primitives::StorageOverlay, _: &mut runtime_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
			with_storage(storage, || {
				let mut check = 0u32;
				for &(ref account, ref shares) in &config.members {
					let temp = check;
					check = temp + shares;
				}
				assert_eq!(&config.pool.1, check);
			});
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: BalanceOf<T>) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");

			// check that too many shares aren't requsted ( 100% is a high upper bound)
			ensure!(shares <= Self::total_shares(), "too many shares requested");

			// check that applicant doesn't have a pending application
			ensure!(!(Self::applicants::exists(&applicant)), "applicant has pending application");

			// check that the TokenTribute covers at least the `ProposalFee`
			ensure!(Self::proposal_fee() >= tokenTribute, "The token tribute does not cover the applicant's required bond");

			// reserve member's bond for proposal
			T::Currency::reserve(&who, Self::proposal_bond())
				.map_err(|_| "balance of proposer is too low")?;
			// reserve applicant's tokenTribute for proposal
			T::Currency::reserve(&applicant, tokenTribute)
				.map_err(|_| "balance of applicant is too low")?;
			
			let time = <system::Module<T>>::block_number;

			let prop = Proposal::new(&who, &applicant, shares, tokenTribute, time);

			Self::proposals::insert(prop.base_hash, prop);
			//add applicant
			Self::applicants::insert(&applicant, prop.base_hash);

			// add yes vote from member who sponsored proposal (and initiate the voting)
			Self::voters_for::mutate(prop.base_hash, |voters| voters.push(&who));
			// supporting map for `remove_proposal`
			Self::proposals::mutate(&who, |props| props.push(prop.base_hash));
			// set that this account has voted
			Self::voter_id::mutate(prop.base_hash, |voters| voters.push(&who));
			// set this for maintainability of other functions
			Self::vote_of::insert(&(prop.base_hash, who), true);
			Self::total_shares_requested::mutate(|count| count + prop.shares);

			Self::deposit_event(RawEvent::Proposed(prop.base_hash, tokenTribute, &who, &applicant));
			Self::deposit_event(RawEvent::Voted(prop.base_hash, true, prop.yesVotes, prop.noVotes));

			Ok(())
		}
		
		/// Allow revocation of the proposal without penalty within the abortWindow
		fn abort(origin, hash: Vec<u8>) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");

			// check if proposal exists
			ensure!(Self::proposals::exists(hash), "proposal does not exist");

			let proposal = Self::proposals::get(hash);

			ensure!(proposal.proposer == &who, "Only the proposer can abort");

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

			Self::deposit_event(RawEvent::Aborted(hash, proposal.tokenTribute, proposal.proposer, proposal.applicant));

			Ok(())
		}

		fn vote(origin, hash: Vec<u8>, approve: bool) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Dao");
			
			ensure!(Self::proposals::exists(hash), "proposal does not exist");

			// load proposal
			let proposal = Self::proposals::get(hash);

			ensure!(
				(proposal.startTime + Self::voting_period::get() >= <system::Module<T>>::block_number()) 
				&& !proposal.passed, 
				format!("The voting period has passed with yes: {} no: {}", proposal.yesVotes, proposal.noVotes)
			);

			// check that member has not yet voted
			ensure!(Self::vote_of::exists(hash, &who), "voter has already submitted a vote on this proposal");

			// note conditional path
			if approve {
				Self::voters_for::mutate(hash, |voters| voters.push(&who));
				Self::proposals_for::insert(&who, |props| props.push(hash));
				Self::voter_id::mutate(hash, |voters| voters.push(&who));
				Self::vote_of::insert(&(hash, who), true);

				// to bound dilution for yes votes
				if Self::total_shares::get() > proposal.maxVotes {
					proposal.maxVotes = Self::total_shares::get();
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

		fn process(origin, hash: Vec<u8>) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");
			ensure!(Self::proposals::exists(hash), "proposal does not exist");

			let proposal = Self::proposals::get(hash);

			// if dilution bound not satisfied, wait until there are more shares before passing
			ensure!(
				Self::total_shares::get().checked_mul(Self::dilution_bound::get()) > proposal.maxVotes, 
				"Dilution bound not satisfied; wait until more shares to pass vote"
			);
			
			let grace_period = (
				(proposal.graceStart <= <system::Module<T>>::block_number()) 
				&& (<system::Module<T>>::block_number() < proposal.graceStart + Self::grace_period::get())
			);
			let pass = proposal.passed;

			if (!grace_period && pass) {

				/// if the proposal passes after it is stale or time expires,
				/// the bonds are forfeited and redistributed to the processer
				// transfer the proposalBond back to the proposer
				T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
				// transfer proposer's proposal bond to the processer
				T::Currency::transfer(&proposal.proposer, &who, Self::proposal_bond());
				// return the applicant's tokenTribute
				T::Currency::unreserve(&proposal.applicant, proposal.tokenTribute);
				// transfer applicant's proposal fee to the processer
				T::Currency::transfer(&proposal.applicant, &who, Self::proposal_fee());

				let late_time = <system::Module<T>>::block_number - (proposal.graceStart + Self::grace_period::get());

				Self::deposit_event(RawEvent::RemoveStale(hash, late_time));
			} else if (grace_period && pass) {
				/// Note: if the proposal passes, the grace_period is started 
				/// (see `fn voted` logic, specifically `if proposal.majority_passed() {}`)
				/// Therefore, this block only executes if the grace_period has proceeded, 
				/// but the proposal hasn't been processed QED

				// transfer the proposalBond back to the proposer
				T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
				// and the applicant's tokenTribute
				T::Currency::unreserve(&proposal.applicant, proposal.tokenTribute);

				// split the proposal fee between the proposer and the processer
				let txfee = Self::proposal_fee().checked_mul(0.5);
				let _ = T::Currency::make_transfer(&proposal.applicant, &who, txfee);
				let _ = T::Currency::make_transfer(&proposal.applicant, &proposal.proposer, txfee);

				let netTribute = proposal.tokenTribute - Self::proposal_fee();

				// transfer tokenTribute to Pool
				let pool = Self::dao_pool();
				let _ = T::Currency::make_transfer(&proposal.applicant, &pool.main, netTribute);

				// mint new shares
				Self::total_shares.mutate(|total| total += proposal.shares);

				// if applicant is already a member, add to their existing shares
				if proposal.applicant.is_member() {
					Self::member_shares::mutate(proposal.applicant, |shares| shares += proposal.shares);
				} else {
					// if applicant is a new member, create a new record for them
					Self::member_shares::insert(proposal.applicant, proposal.shares);
					Self::active_members::mutate(|mems| mems.push(proposal.applicant));
					Self::member_count::mutate(|count| count + 1);
					pool.shares += proposal.shares;
				}
			} else {
				/// proposal did not pass
				/// send all bonds to the processer
				// transfer the proposalBond back to the proposer
				T::Currency::unreserve(&proposal.proposer, Self::proposal_bond());
				// transfer proposer's proposal bond to the processer
				T::Currency::transfer(&proposal.proposer, &who, Self::proposal_bond());
				// return the applicant's tokenTribute
				T::Currency::unreserve(&proposal.applicant, proposal.tokenTribute);
				// transfer applicant's proposal fee to the processer
				T::Currency::transfer(&proposal.applicant, &who, Self::proposal_fee());
				
			}
			
			Self::remove_proposal(hash);
		
			Self::deposit_event(RawEvent::Processed(hash, proposal.tokenTribute, proposal.applicant, proposal.passed));

			Ok(())
		}

		fn rage_quit(origin, sharesToBurn: u32) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of the DAO");

			let shares = Self::member_shares::get(&who);
			ensure!(shares >= sharesToBurn, "insufficient shares");

			// check that all proposals have passed
			//
			// this would be in `poll` (for async)
			ensure!(Self::proposals_for::get(&who).iter()
					.all(|prop| prop.passed && 
						(<system::Module<T>>::block_number() <= prop.graceStart + Self::grace_period())
					), "All proposals have not passed or exited the grace period"
			);

			Self::dao_pool().withdraw(&who, sharesToBurn);

			// update DAO Membership Maps
			Self::member_count::set(|count| count - 1);
			Self::active_members::mutate(|mems| mems.retain(|&x| x != &who));
			Self::member_shares::remove(&who);

			Ok(())
		}
	}
}

impl<AccountId, Balance, BlockNumber> Default for Proposal<AccountId, Balance, BlockNumber> {
	fn default() -> Self {
		Proposal {
			base_hash: 0,		// should be set manually
			proposer: 1,		// should ""
			applicant: 2,		// should ""
			shares: 10,			// should ""
			startTime: 11,		// should ""
			graceStart: None,	// can	  ""
			yesVotes: 0,		// can	  ""
			noVotes: 0,			// can	  ""
			maxVotes: 0,		// should ""
			processed: false,	// can 	  ""
			passed: false,		// can 	  ""
			tokenTribute: 0,	// should ""
		}
	}
}

impl<AccountId, Balance, BlockNumber> Proposal<AccountId, Balance, BlockNumber> {
	pub fn new(proposer: AccountId, applicant: AccountId, shares: u32, tokenTribute: Balance, time: BlockNumber) -> Self {
		let base = Base {
			proposer: &proposer,
			applicant: &applicant,
			shares: shares,
			tokenTribute: tokenTribute
		};

		let hash = base.encode();
		// ensure a proposal with the same UID encoding doesn't exist
		ensure!(!(Self::proposals::exists(hash)), "Key collision ;(");

		let yesVotes = Self::member_shares::get(&proposer);
		let maxVotes = Self::total_shares();
		Self::total_shares::mutate(|count| count + shares);

		Proposal {
			base_hash: hash,
			proposer: &proposer,
			applicant: &applicant,
			shares: shares,
			startTime: time,
			yesVotes: yesVotes,
			maxVotes: maxVotes,
			tokenTribute: tokenTribute,
			..Default::default()
		}

	}


	// TODO (abstract voting algorithms into their own file)
	// more than half shares voted yes
	pub fn majority_passed(&self) -> bool {
		// do I need the `checked_div` flag?
		if self.maxVotes % 2 == 0 { 
			return (self.yesVotes > self.maxVotes.checked_div(2)) 
		} else { 
			return (self.yesVotes > (self.maxVotes.checked_add(1).checked_div(2)))
		};
	}
	// ADD multiple voting algorithms so the DAO can choose which one to use
	// - QV (based on shares for each voters)
	// - AQP (based on shares and turnout)
	// - increased weight to voters who haven't voted for a while (like an AQP extension)
}

impl<AccountId> Pool<AccountId> {
	pub fn withdraw(&self, receiver: AccountId, sharedBurned: u32) -> Result { 
		// Checks on identity made in `rage_quit` (the only place in which this is called)

		let amount = Currency::free_balance(&self.main).checked_mul(sharedBurned).checked_div(Self::total_shares());
		let _ = Currency::make_transfer(&self.main, &receiver, amount)?;

		self.shares -= sharedBurned;

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

	/// Clean up storage maps involving proposal
	pub fn remove_proposal(hash: Vec<u8>) -> Result {
		ensure!(Self::proposals::exists(hash), "the given proposal does not exist");
		let proposal = Self::proposals::get(hash);
		Self::proposals::remove(hash);
		Self::applicants::remove(proposal.applicant);

		let voters = Self::voters_for::get(hash).iter().map(|voter| {
			Self::proposals_for::mutate(&voter, |hashes| hashes.iter().filter(|hush| hush != hash).collect());
			voter
		});

		Self::voter_id::remove(hash);
		Self::voters_for::remove(hash);
		// reduce outstanding share request amount
		Self::total_shares_requested::set(|total| total -= proposal.shares);

		Ok(())
	} 
}