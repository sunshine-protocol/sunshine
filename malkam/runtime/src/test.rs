/// tests for this module

/// DO THIS LAST

/// IMPORTANT:
/// TEST TIMING ATTACKS ON ACCEPTANCE AND RAGE QUITTING
/// RIGOROUS TESTING AGAINST OVERFLOW/UNDERFLOW FOR SHARE REQUEST AND OTHER PARAMETERS

/// TEMP FOR CODING WITHOUT DUAL MONITOR

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
		Proposals get(proposals): map ProposalIndex => Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber>>;
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






/// Abstract into test.rs for cleanliness
/// Might need to use a mock.rs for setup
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<u64>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	impl Trait for Test {
		type Event = ();
	}
	type Malkam = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(Malkam::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(Malkam::something(), Some(42));
		});
	}
}
