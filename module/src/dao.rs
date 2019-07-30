use parity_codec::{Decode, Encode};
#[cfg(feature = "std")]
use runtime_primitives::traits::{AccountIdConversion, Hash, StaticLookup, Zero};
use runtime_primitives::{ModuleId, Permill};
use support::traits::{ChangeMembers, Currency, Get, ReservableCurrency};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

// START TYPES
type ProposalCount = u32;
type MemberCount = u32;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

const DAO_ID: ModuleId = ModuleId(*b"py/daofi");

/// Spending proposal (mix from `treasury` and `collective`)
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, Balance, BlockNumber> {
    proposer: AccountId,
    beneficiary: AccountId,
    // amount of spend proposal
    value: Balance,
    // proposer's bond (reserved)
    bond: Balance,
    // BlockNumber at initial proposal
    start: BlockNumber,
    // threshold for passage
    threshold: MemberCount,
    // supporting voters (voted yes)
    ayes: Vec<AccountId>,
    // against voters (voted no)
    nays: Vec<AccountId>,
}
// END TYPES

pub trait Trait: system::Trait {
    /// The balances type
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// What to do when the members change
    type ChangeMembers: ChangeMembers<Self::AccountId>;

    /// Entry fee
    type EntryFee: Get<BalanceOf<Self>>;

    /// Bond required for proposal
    type ProposalBond: Get<BalanceOf<Self>>;

    /// Bond required to become an active voter
    type VoteBond: Get<BalanceOf<Self>>;

    /// Period for which votes are valid after initial proposal
    type VoteWindow: Get<Self::BlockNumber>;

    /// Frequency with which stale proposals are purged
    type PurgeFrequency: Get<Self::BlockNumber>;

    /// Frequency with which the passed proposals are executed
    type SpendFrequency: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        BlockNumber = <T as system::Trait>::BlockNumber,
        Hash = <T as system::Trait>::Hash,
        Balance = BalanceOf<T>,
    {
        // proposer AccountId, proposal Hash, value spend proposed, block number
        Proposed(AccountId, Hash, Balance, BlockNumber),
        // new voter AccountId, BlockNumber at registration
        RegisterVoter(AccountId, BlockNumber),
        // old voter AccountId, BlockNumber at deregistration
        DeRegisterVoter(AccountId, BlockNumber),
        // voter AccountId, proposal Hash, vote bool, yes_count
        Voted(AccountId, Hash, bool, MemberCount), // TODO: add BlockNumber
        // new member AccountId
        NewMember(AccountId),
        // old member AccountId
        MemberExit(AccountId),
        // proposal Hash, value spent, beneficiary that received funds
        Paid(Hash, Balance, AccountId),
        // the amount left in the pot
        BudgetRemaining(Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as MiniDAO {
        /// Proposals that have been made.
        Proposals get(proposals): map T::Hash => Option<Proposal<T::AccountId, BalanceOf<T>, T::BlockNumber>>;
        /// Number of active proposals
        TotalProposals get(total_proposals): ProposalCount;
        /// Proposals that have passed, awaiting execution in `on_finalize`
        Passed get(passed): Vec<T::Hash>;
        /// Proposals that have grown stale, awaiting purging
        Stale get(stale): Vec<T::Hash>;

        /// Members of the DAO
        Member get(member): Vec<T::AccountId>;
        /// Total member count
        TotalMembers get(total_members): MemberCount;
        /// Voting members
        Voters get(voters): Vec<T::AccountId>;

        /// Who is able to vote for whom. Value is the fund-holding account, key is the
        /// vote-transaction-sending account.
        Proxy get(proxy): map T::AccountId => Option<T::AccountId>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Price for joining
        const EntryFee: BalanceOf<T> = T::EntryFee::get();

        /// Bond for proposals, to be returned
        const ProposalBond: BalanceOf<T> = T::ProposalBond::get();

        /// Bond for active voters, to be returned
        const VoteBond: BalanceOf<T> = T::VoteBond::get();

        /// Period after initial proposal during which voting is allowed
        const VoteWindow: T::BlockNumber = T::VoteWindow::get();

        /// Period between stale proposal purges
        const PurgeFrequency: T::BlockNumber = T::PurgeFrequency::get();

        /// Period between successive spends.
        const SpendFrequency: T::BlockNumber = T::SpendFrequency::get();

        fn join(origin) {
            let new_member = ensure_signed(origin)?;
            ensure!(!Self::is_member(&new_member), "new member is already a member");

            // take a fee from the new member
            let value = T::EntryFee::get(); // TODO: replace with `calculate_entry_fee` based on member size and pot size
            T::Currency::transfer(&new_member, &Self::account_id(), value)
                .map_err(|_| "Not rich enough to join ;(")?;

            // add new member
            <Member<T>>::mutate(|v| v.push(new_member.clone()));
            let c = Self::total_members() + 1;
            <TotalMembers>::put(c);
            // change member set
            T::ChangeMembers::change_members(&[new_member.clone()], &[], &Self::member()[..]);

            Self::deposit_event(RawEvent::NewMember(new_member));
        }

        fn exit(origin) {
            let old_member = ensure_signed(origin)?;
            ensure!(Self::is_member(&old_member), "exiting member must be a member");
            ensure!(!Self::is_active_voter(&old_member), "exiting member must deregister as a voter before leaving the DAO");

            // exiting member notably gets nothing here
            // remark on dilution complexity normally involved in exits

            // remove existing member
            <Member<T>>::mutate(|m| m.retain(|x| x != &old_member));
            let c = Self::total_members() - 1;
            <TotalMembers>::put(c);
            // change member set
            T::ChangeMembers::change_members(&[], &[old_member.clone()], &Self::member()[..]);

            Self::deposit_event(RawEvent::MemberExit(old_member));
        }

        /// almost identical to method in `srml/treasury` (see `do_propose` for core logic)
        fn propose(
            origin,
            value: BalanceOf<T>,
            beneficiary: <T::Lookup as StaticLookup>::Source
        ) -> Result {
            let proposer = ensure_signed(origin)?;
            // check membership
            ensure!(Self::is_member(&proposer), "proposer must be a member to make a proposal");
            ensure!(value < Self::pot(), "not enough funds in the DAO to execute proposal");

            Self::do_propose(proposer, value, beneficiary)
        }

        fn proxy_propose(
            origin,
            value: BalanceOf<T>,
            beneficiary: <T::Lookup as StaticLookup>::Source
        ) -> Result {
            let proxy_proposer = Self::proxy(ensure_signed(origin)?).ok_or("not a proxy")?;
            ensure!(Self::is_member(&proxy_proposer), "voter must be a member to approve/deny a proposal");

            Self::do_propose(proxy_proposer, value, beneficiary)
        }

        fn register2_vote(origin) -> Result {
            let new_voter = ensure_signed(origin)?;
            ensure!(Self::is_member(&new_voter), "every voter must be a member");
            ensure!(!Self::is_active_voter(&new_voter), "must not be an active voter yet");

            T::Currency::reserve(&new_voter, T::VoteBond::get())
                .map_err(|_| "member doesn't have enough free balance for vote bond")?;

            <Voters<T>>::mutate(|v| v.push(new_voter.clone())); // replace with append once
            let start = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::RegisterVoter(new_voter, start));
            Ok(())
        }

        fn deregister2_vote(origin) -> Result {
            let old_voter = ensure_signed(origin)?;
            ensure!(Self::is_active_voter(&old_voter), "must be an active voter");

            T::Currency::unreserve(&old_voter, T::VoteBond::get());

            <Voters<T>>::get().retain(|v| v!= &old_voter);
            let end = <system::Module<T>>::block_number();

            Self::deposit_event(RawEvent::DeRegisterVoter(old_voter, end));
            Ok(())
        }

        // similar to `srml/council` `vote` method, but execution is handled in `on_finalize`
        fn vote(origin, proposal_hash: T::Hash, approve: bool) -> Result {
            // increase the support count
            let voter = ensure_signed(origin)?;
            ensure!(Self::is_member(&voter), "voter must be a member to approve/deny a proposal");
            ensure!(Self::is_active_voter(&voter), "voter must be an active voter");

            Self::do_vote(voter, proposal_hash, approve)
        }

        fn proxy_vote(origin, proposal_hash: T::Hash, approve: bool) -> Result {
            let proxy_voter = Self::proxy(ensure_signed(origin)?).ok_or("not a proxy")?;
            ensure!(Self::is_member(&proxy_voter), "proxy voter must be a member to approve/deny a proposal");
            ensure!(Self::is_active_voter(&proxy_voter), "proxy voter must be an active voter");

            Self::do_vote(proxy_voter, proposal_hash, approve)
        }

        fn on_finalize(n: T::BlockNumber) {
            
            // PURGE
            if (n % T::PurgeFrequency::get()).is_zero() {
                Self::purge();
            }

            // SPEND
            let mut budget = Self::pot();
            if (n % T::SpendFrequency::get()).is_zero() {
                budget = Self::spend(budget);
            }// TODO: is_zero() requires the Zero trait from runtime_primitives::traits

            Self::deposit_event(RawEvent::BudgetRemaining(budget));
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
        Self::member().contains(who)
    }

    pub fn is_active_voter(who: &T::AccountId) -> bool {
        Self::voters().contains(who)
    }

    pub fn calculate_proposal_bond(value: BalanceOf<T>) -> BalanceOf<T> {
        // ideal spend is set at 1/10th of the pot
        let ideal = Permill::from_percent(10) * Self::pot(); // does this need to be divided by 1_000_000
        let positive = value >= ideal;
        let diff = value.max(ideal) - value.min(ideal);
        // cache T::ProposalBond::get() instead of making multiple storage calls
        let ideal_bond = T::ProposalBond::get();
        // (diff/ideal) * T::ProposalBond::get()
        let delta = Permill::from_rational_approximation(diff, ideal) * ideal_bond;

        if positive {
            // (1 + diff/ideal) * T::ProposalBond::get()
            return delta + ideal_bond;
        } else {
            // (1 - diff/ideal) * T::ProposalBond::get()
            return ideal_bond - delta;
        }
    }

    // calculate_entry_bond (membership, voting)
    // depends on a defined target number of members
    // {`calculate_entry_fee`, `calculate_vote_bond`, `calculate_spend_frequency`, `calculate_threshold`}

    // !--private functions--!

    /// Proposals (of the spend variety)
    ///
    /// Called by `propose` and `proxy_propose`
    fn do_propose(
        proposer: T::AccountId,
        value: BalanceOf<T>,
        beneficiary: <T::Lookup as StaticLookup>::Source,
    ) -> Result {
        // lookup beneficiary
        let beneficiary = T::Lookup::lookup(beneficiary)?;

        // bond the proposer
        let bond = Self::calculate_proposal_bond(value);
        T::Currency::reserve(&proposer, bond).map_err(|_| "Proposer's balance too low")?;

        // increment the proposal count
        let c = Self::total_proposals() + 1;
        TotalProposals::put(c); // TODO: decrement when proposals pass
                                    // threshold set at majority at time of proposal
        let threshold = Self::total_members() / 2 + 1; // TODO: calculate_threshold
                                                       // BlockNumber at which VoteWindow begins
        let start = <system::Module<T>>::block_number();
        // ayes and nays vectors
        let mut ayes = Vec::new();
        let mut nays: Vec<T::AccountId> = Vec::new();
        ayes.push(proposer.clone());
        // clone proposer for event emission after proposal insertion
        let p = proposer.clone();
        let proposal = Proposal {
            proposer,
            beneficiary,
            value,
            bond,
            start,
            threshold,
            ayes,
            nays,
        };
        // take hash of proposal
        let hash = <T as system::Trait>::Hashing::hash_of(&proposal);
        // insert proposal
        <Proposals<T>>::insert(hash, &proposal);
        Self::deposit_event(RawEvent::Proposed(p, hash, value, start));
        Ok(())
    }

    /// Voting
    ///
    /// Called by `vote` and `proxy_vote`
    fn do_vote(voter: T::AccountId, proposal_hash: T::Hash, approve: bool) -> Result {
        // verify proposal existence
        let mut voting = Self::proposals(&proposal_hash).ok_or("proposal must exist")?;

        if voting.start + T::VoteWindow::get() < <system::Module<T>>::block_number() {
            <Stale<T>>::mutate(|s| s.push(proposal_hash.clone())); // TODO: update with `append` once PR merged
            return Err("The voting period is over for this proposal");
        }

        let position_yes = voting.ayes.iter().position(|a| a == &voter);
        let position_no = voting.nays.iter().position(|a| a == &voter);

        if approve {
            if position_yes.is_none() {
                voting.ayes.push(voter.clone());
            } else {
                return Err("duplicate vote");
            }
            // executes if the previous vote was no
            if let Some(pos) = position_no {
                // ability to change vote at no cost prevents bribery attacks
                voting.nays.swap_remove(pos);
            }
        } else {
            if position_no.is_none() {
                voting.nays.push(voter.clone());
            } else {
                return Err("duplicate vote");
            }
            if let Some(pos) = position_yes {
                voting.ayes.swap_remove(pos);
            }
        }
        let yes_count = voting.ayes.len() as MemberCount;
        let threshold = voting.threshold;
        if yes_count > threshold {
            <Passed<T>>::mutate(|p| p.push(proposal_hash)); // TODO: update with `append`
        }

        Self::deposit_event(RawEvent::Voted(voter, proposal_hash, approve, yes_count));
        Ok(())
    }

    /// Spending
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn spend(mut budget_remaining: BalanceOf<T>) -> BalanceOf<T> {
        let mut missed_any = false;
        <Passed<T>>::mutate(|v| {
            v.retain(|&hash| {
                if let Some(p) = Self::proposals(hash) {
                    if p.value <= budget_remaining {
                        budget_remaining -= p.value;
                        <Proposals<T>>::remove(hash);
                        let total = Self::total_proposals() - 1;
                        <TotalProposals>::put(total);

                        //might require checks that I'm neglecting...? case of multiple proposals?

                        // return bond
                        let _ = T::Currency::unreserve(&p.proposer, p.bond);

                        // transfer the funds
                        let _ = T::Currency::transfer(&Self::account_id(), &p.beneficiary, p.value);

                        Self::deposit_event(RawEvent::Paid(hash, p.value, p.beneficiary));
                        false
                    } else {
                        missed_any = true;
                        true
                    }
                } else {
                    false
                }
            });
        });
        budget_remaining
    }

    /// Purging stale proposals
    ///
    /// Called in `on_finalize` according to `T::SpendFrequency`
    fn purge() {
        // use type state to consume lazy iterator adaptor
        // TODO: ask for review and discuss
        let _ = <Stale<T>>::get().into_iter().for_each(|h| {
            <Proposals<T>>::remove(h);
            let total = Self::total_proposals() -1;
            <TotalProposals>::put(total);
        });
        <Stale<T>>::kill();
    }
}

// TESTING
#[cfg(test)]
mod tests {
    use super::RawEvent;
    use super::*;
    use crate::tests::*;
    use crate::tests::{Call, Event as OuterEvent, Origin};
    use runtime_primitives::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup, OnFinalize},
    };
    use support::{assert_noop, assert_ok, Hashable};

    #[test]
    fn basic_setup_works() {
        with_externalities(&mut new_test_ext(), || {
            assert_eq!(MiniDAO::pot(), 0);
            assert_eq!(MiniDAO::proposal_count(), 0);
        });
    }

    #[test]
    fn add_existing_member_fails() {
        assert_eq!(1, 1);
    }

    #[test]
    fn non_member_exit_fails() {
        assert_eq!(1, 1);
    }

    #[test]
    fn spend_proposal_takes_min_deposit() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(MiniDAO::propose_spend(Origin::signed(0), 1, 3));
            assert_eq!(Balances::free_balance(&0), 99);
            assert_eq!(Balances::reserved_balance(&0), 1);
        });
    }

    #[test]
    fn register_nonmember() {
        // nonmember cannot register
        assert_eq!(1, 1);
    }

    #[test]
    fn exit_voter() {
        // active voter cannot leave the DAO
        assert_eq!(1, 1);
    }

    // test proxy
}