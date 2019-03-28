/// TODO: square away imports in `lib.rs` and `Cargo.toml`
/// (do this after I finish implementing the relevant logic)

// these two compilation flags were at the top of `treasury` ¯\_(ツ)_/¯ 
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use primitives::traits::{Zero, As, Bounded}; // don't really use these
use parity_codec::{Encode, Decode};
use support::{StorageValue, StorageMap, Parameter, Dispatchable, IsSubType, EnumerableStorageMap}; // don't use all of these...
use support::{decl_module, decl_storage, decl_event, ensure};
use support::traits::{Currency, OnUnbalanced}; // WithdrawReason, LockIdentifier, LockableCurrency
use support::dispatch::Result;
use system::ensure_signed;

/// for counting votes (like safemath kind of?) 
/// WHEN DO I USE THESE? 
use primitives::traits::{Zero, IntegerSquareRoot, Hash}; // repeated import of Zero
use rstd::ops::{Add, Mul, Div, Rem};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
// presumably for slashing the member who proposed something that is rejected?
type NegativeImbalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::NegativeImbalance;

/// Our module's configuration trait. All our types and consts go in here. If the
/// module is dependent on specific other modules, then their configuration traits
/// should be added to our implied traits list.
///
/// `system::Trait` should always be included in our implied traits.
pub trait Trait: system::Trait {
	// the staking balance (primarily for bonding applications)
	type Currency: Currency<Self::AccountId>;

	/// Handler for the unbalanced decrease when slashing for a rejected proposal. (from `Treasury`)
	type ProposalRejection: OnUnbalanced<NegativeImbalanceOf<Self>>;

	// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// Use this to increase the readability for Proposal State Transitions
/// (maintainability/readability as a criteria)
/// (common state machine pattern)
pub enum PropState {
	Voting,				// undergoing voting (and not yet aborted)
	Aborted,			// aborted during the voting period
	Processable, 		// grace period (can be processed at this time)
	Executed,			// successfully processed and executed
}

/// FROM `TREASURY`
type ProposalIndex = u32;
/// A proposal to lock up tokens in exchange for shares
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Proposal<AccountId, Balance, BlockNumber: Parameter> {
	proposer: AccountId,			// proposer AccountId
	applicant: AccountId,			// applicant AccountId
	shares: u32, 					// number of requested shares
	tokenTribute: Balance, 			// tokenTribute
	startTime: BlockNumber,			// when the voting period starts
	state: PropState,				// the state of the Proposal
	threshold: u32,					// number of shares necessary to approve (greater than half at first)
}
/// DO WE WANT TO ADD A THRESHOLD FLAG SET UPON INSTANTIATION BASED ON OUSTANDING SHARES? YES
/// REORDER THE PARAMETERS 
/// ADD THEM TO PROPOSE FUNCTION FOR INSTANTIATION IN PROPOSE (specifically `threshold`)

decl_event!(
	/// An event in this module.
	pub enum Event<T> 
	where
		<T as system::Trait>::AccountId 
	{
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
		// switch the identity, using the old key
		UpdateDelegateKey(AccountId), // this is really only necessary if we don't use the member's address as the default
		// Do we need a config event?
		// SummonComplete(address indexed summoner, uint256 shares);
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// (Re-)configure this module. (UPDATE WITH PARAMETERS)
		/// TODO
		fn configure(
			#[compact] proposal_bond: Permill,
			#[compact] proposal_bond_minimum: BalanceOf<T>,
			#[compact] spend_period: T::BlockNumber,
			#[compact] burn: Permill
		) {
			<ProposalBond<T>>::put(proposal_bond);
			<ProposalBondMinimum<T>>::put(proposal_bond_minimum);
			<SpendPeriod<T>>::put(spend_period);
			<Burn<T>>::put(burn);
		}

		fn propose(origin, applicant: AccountId, shares: u32, tokenTribute: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Malkam DAO");

			// check that too many shares aren't requsted (< max set in config)
			ensure!(shares <= Self::max_shares(), "too many shares requested");

			// check that applicant doesn't have a pending application
			ensure!(!<Applicants>::exists(&applicant), "applicant has pending application");

			// reserve member's bond for proposal
			T::Currency::reserve(&who, Self::proposal_bond_minimum())
				.map_err(|_| "proposer's balance too low")?;

			// add proposal (TODO: TEST CORRECT INDEXING HERE)
			let count = Self::proposal_count(); // how does this actually work? Must config correctly!
			<ProposalCount<T>>::put(count + 1);
			<Applicants<T>::insert(&applicant, count);
			let startTime = <system::Module<T>>::block_number();
			// one concern I have is if I can just put Voting as propState like this or if I need some prior variable assignment
			<Proposals<T>>::insert(count, Proposal { who, applicant, shares, tokenTribute, startTime, Voting });

			Self::deposit_event(RawEvent::Proposed(count));
		}

		// enable the member who made a proposal to abort
		// think long and hard about timing attacks
		fn abort() {
		}

		fn vote(origin, proposal: ProposalIndex, approve: bool) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_member(&who), "proposer is not a member of Malkam DAO");
			
			ensure!(<Proposals<T>>::exists(proposal), "proposal does not exist");

			// check that the vote is made within the voting period (TODO: can you compare times with `<=` operator?)
			ensure!(<system::Module<T>>::block_number() <= <Proposals<T>>::get(proposal).startTime + <Malkam<T>>::voting_period(), "it is past the voting period")
			
			// check that the proposal is not aborted
			ensure!(<Proposals<T>::get(proposal).state == abort, "The proposal has been aborted");

			// check that the member has not yet voted
			ensure(!<Malkam<T>>::voter_id(proposal).iter().any(|x| &x == &who), "member has already voted on the proposal");

			if approve {

			} else {

			}
			// (1) change the highestIndex for member to prevent preemptive ragequitting
			// (2) set maximum total shares for yes vote (to bound dilution for yes voters)?
			// (3) add to yes votes total (according to number of shares)
			// (4) add to voter_id map from ProposalIndex `=>` Vec<AccountId>

			// if no =>
			// add to no votes total (according to number of shares)

			Self::deposit_event(RawEvent::Voted(ProposalIndex, approve, yes_count, no_count));
		}

		fn process() {

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
		/// CONFIG (like the constructor values)
		VotingPeriod get(voting_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // convert from block numbers to days (currently just 7 days)
		GracePeriod get(grace_period) config(): T::BlockNumber = T::BlockNumber::sa(7); // ""  
		AbortWindow get(abort_window) config(): T::BlockNumber = T::BlockNumber::sa(1); // "" 1 day
		// Amount of funds that must be put at stake (by a member) for making a proposal. (0.1 ETH in MolochDAO)
		ProposalBond get(proposal_bond_minimum) config(): BalanceOf<T>;
		// Maximum number of shares that can be requested for any proposal
		MaxSharesRequested get(max_shares) config(): u32;

		/// TRACKING PROPOSALS
		// Proposals that have been made.
		Proposals get(proposals): map ProposalIndex => Option<Proposal<T::AccountId, BalanceOf<T>>>;
		// Active Applicants (to prevent multiple applications at once)
		Applicants get(applicants): map T::AccountId => Option<ProposalIndex>; // may need to change to &T::AccountId
		// Number of proposals that have been made.
		ProposalCount get(proposal_count): ProposalIndex;

		/// VOTING
		// to protect against rage quitting
		HighestYesIndex get(highest_yes_index): map T::AccountId => Option<ProposalIndex>;
		// map: proposalIndex => Voters that have voted
		VoterId get(voter_id): map ProposalIndex => Vec<AccountId>;


		// pub ProposalOf get(proposal_of): map T::Hash => Option<T::Proposal>;
		// pub ProposalVoters get(proposal_voters): map T::Hash => Vec<T::AccountId>;
		// pub VetoedProposal get(veto_of): map T::Hash => Option<(T::BlockNumber, Vec<T::AccountId>)>;

		/// DAO MEMBERSHIP - permanent state (always relevant, changes only at the finalisation of voting)
		ActiveMembers get(active_members) config(): Vec<T::AccountId>; // the current DAO members
		MemberShares get(member_shares): map T::AccountId => u32; // shares of the current DAO members

		/// INTERNAL ACCOUNTING
		// Number of shares across all members
		TotalShares get(total_shares) config(): u32; 
		// total shares that have been requested in unprocessed proposals
		TotalSharesRequested get(total_shares_requested): u32; 

		/// DELEGATE_KEY
		// applicant => member (because proposals are only made by members)
		DelegatedMember get(delegated_member): T::AccountId => T::AccountId; // probably delete; unncessary
	}
}

impl<T: Trait> Module<T> {
	pub fn is_member(who: &T::AccountId) -> bool {
		<Malkam<T>>::active_members().iter()
			.any(|&(ref a, _)| a == who)
	}

	pub fn can_quit(who: &T::AccountId) -> bool {
		/// use mapping: members => highestIndexYesVote
	}
}

/// tests in another file for ease of readability...could put them back here depending on the file length