/// TODO: square away imports in `lib.rs` and `Cargo.toml`
/// (do this after I finish implementing the relevant logic)
use primitives::traits::{Zero, As, Bounded};
use parity_codec::{Encode, Decode};
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap};
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency, LockableCurrency, WithdrawReason, LockIdentifier, OnUnbalanced};
use support::dispatch::Result;
use system::ensure_signed;

/// for counting votes (like safemath kind of?) 
/// WHEN DO I USE THESE? 
use primitives::traits::{Zero, IntegerSquareRoot, Hash};
use rstd::ops::{Add, Mul, Div, Rem};

type NegativeImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

/// Our module's configuration trait. All our types and consts go in here. If the
/// module is dependent on specific other modules, then their configuration traits
/// should be added to our implied traits list.
///
/// `system::Trait` should always be included in our implied traits.
pub trait Trait: system::Trait {
	// Lockable Currency (for staking-based voting)
	type Currency: LockableCurrency<<Self as system::Trait>::AccountId, Moment=Self::BlockNumber>;

	// Proposal (USE A STRUCT INSTEAD)
	// type Proposal: Parameter + Dispatchable<Origin=Self::Origin> + IsSubType<Module<Self>>

	/// Handler for the unbalanced decrease when slashing for a rejected proposal.
	type ProposalRejection: OnUnbalanced<NegativeImbalanceOf<Self>>;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// FROM `COUNCIL`
/// Origin for the malkam module.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Origin {
	/// It has been accepted by a given number of the DAO members.
	Members(u32),
}

/// FROM `TREASURY`
type ProposalIndex = u32;
/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance> {
	proposer: AccountId,			// proposer AccountId
	applicant: AccountId,
	shares: u32, 					// number of requested shares
	tokenTribute: Balance, 			// tokenTribute
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> 
	where 
		<T as system::Trait>::Hash, 
		<T as system::Trait>::AccountId 
	{
		/// A proposal has been submitted 
		Proposed(AccountId),
		/// A proposal has been voted on by given account, leaving
		/// a tally (yes votes and no votes given as u32s respectively).
		Voted(AccountId, Hash, bool, u32, u32),
		/// A proposal was approved by the required threshold.
		Approved(Hash),
		/// A proposal was not approved by the required threshold.
		Rejected(Hash),
		/// The proposal was processed (executed); `bool` is true if returned without error
		Processed(Hash, bool),
		// The member `ragequit` the DAO
		// TODO: NEED TO PROTECT AGAINST TIMING ATTACKS FOR THIS...
		Ragequit(AccountId), 
		// switch the identity, using the old key
		UpdateDelegateKey(AccountId), // this is really only necessary if we don't use the member's address as the default
		// Abort(uint256 indexed proposalIndex, address applicantAddress);
		// SummonComplete(address indexed summoner, uint256 shares);
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		// using `council/motion.rs` && `treasury/lib.rs`
		fn propose(
			origin, 
			applicant: AccountId,
			tokenTribute: Balance,

		) {
			// check that too many shares aren't requested (need to set max shares thing; maybe in initial config)
			
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Moloch DAO");

			// instantiate proposal and see if one exists (from applicant)?

			// check if proposal already exists

			let count = Self::proposal

			// event: <Proposed>
		}

		fn propose_spend(
			origin,
			#[compact] value: BalanceOf<T>,
			beneficiary: <T::Lookup as StaticLookup>::Source
		) {
			let proposer = ensure_signed(origin)?;
			let beneficiary = T::Lookup::lookup(beneficiary)?;

			let bond = Self::calculate_bond(value);
			T::Currency::reserve(&proposer, bond)
				.map_err(|_| "Proposer's balance too low")?;

			let c = Self::proposal_count();
			<ProposalCount<T>>::put(c + 1);
			<Proposals<T>>::insert(c, Proposal { proposer, value, beneficiary, bond });

			Self::deposit_event(RawEvent::Proposed(c));
		}

		fn vote() {
		}

		fn process() {
		}

		fn rage_quit() {
		}

		/// Implementation Borrowed from Sudo
		///
		/// for UpdateDelegateKey (wtf is `<T::Lookup as StaticLookup>::Source`)
		fn set_key(origin, new: <T::Lookup as StaticLookup>::Source) {
			// This is a public call, so we ensure that the origin is some signed account.
			let sender = ensure_signed(origin)?;
			ensure!(sender == Self::key(), "only the current delegate key can change the key");
			let new = T::Lookup::lookup(new)?;

			Self::deposit_event(RawEvent::UpdateDelegateKey(Self::key()));
			<Key<T>>::put(new);
		}
	}
}

/// some taken from `council/seats.rs` (useful for keeping track of members)
decl_storage! {
	trait Store for Module<T: Trait> as Malkam {
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(3);
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(1000);
		/// tracking Proposals
		/// Proposals that have been made.
		Proposals get(proposals): map ProposalIndex => Option<Proposal<T::AccountId, BalanceOf<T>>>;

		// pub ProposalOf get(proposal_of): map T::Hash => Option<T::Proposal>;
		// pub ProposalVoters get(proposal_voters): map T::Hash => Vec<T::AccountId>;
		// pub VetoedProposal get(veto_of): map T::Hash => Option<(T::BlockNumber, Vec<T::AccountId>)>;

		/// DAO MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current DAO members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current DAO members
		/// INTERNAL ACCOUNTING
		TotalShares get(total_shares) config(): u32; // Number of shares across all members
		TotalSharesRequested get(total_shares_requested): u32; // total shares that have been requested in unprocessed proposals
		/// DELEGATE_KEY
		// applicant => member (because proposals are only made by members)
		DelegatedMember get(delegated_member): T::AccountId => T::AccountId;
		Key get(key) config(): T::AccountId;
	}
}

impl<T: Trait> Module<T> {
	pub fn is_member(who: &T::AccountId) -> bool {
		<Malkam<T>>::active_members().iter()
			.any(|&(ref a, _)| a == who)
	}
}

/// tests in another file for ease of readability...could put them back here depending on the file length