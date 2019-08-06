#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]
pub mod dao;
pub mod test;

pub use dao::{Call, Event, Module, RawEvent, Trait};

// test scaffolding
#[cfg(test)]
mod tests {
    pub use super::*;
    pub use dao;
    pub use primitives::{Blake2Hasher, H256};
    pub use runtime_io::with_externalities;
    pub use runtime_primitives::{
        testing::{Digest, DigestItem, Header},
        traits::{BlakeTwo256, IdentityLookup, OnFinalize},
        BuildStorage, Permill, Perbill,
    };
    use support::{impl_outer_origin, parameter_types};

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    thread_local! {
        static PROPOSAL_BOND: RefCell<u64> = RefCell::new(3);
        static VOTE_BOND: RefCell<u64> = RefCell::new(5);
        static APPLICATION_WINDOW: RefCell<u64> = RefCell::new(4);
        static ABORT_WINDOW: RefCell<u64> = RefCell::new(6);
        static VOTE_WINDOW: RefCell<u64> = RefCell::new(10);
        static GRACE_WINDOW: RefCell<u64> = RefCell::new(10);
        static SWEEP_FREQUENCY: RefCell<u64> = RefCell::new(10);
        static ISSUANCE_FREQUENCY: RefCell<u64> = RefCell::new(10);
    }

    pub struct ProposalBond;
    impl Get<u64> for ProposalBond {
        fn get() -> u64 { PROPOSAL_BOND.with(|v| *v.borrow()) }
    }

    pub struct VoteBond;
    impl Get<u64> for VoteBond {
        fn get() -> u64 { VOTE_BOND.with(|v| *v.borrow()) }
    }

    pub struct ApplicationWindow;
    impl Get<u64> for ApplicationWindow {
        fn get() -> u64 { APPLICATION_WINDOW.with(|v| *v.borrow()) }
    }

    pub struct AbortWindow;
    impl Get<u64> for AbortWindow {
        fn get() -> u64 { ABORT_WINDOW.with(|v| *v.borrow()) }
    }

    pub struct VoteWindow;
    impl Get<u64> for VoteWindow {
        fn get() -> u64 { VOTE_WINDOW.with(|v| *v.borrow()) }
    }

    pub struct GraceWindow;
    impl Get<u64> for GraceWindow {
        fn get() -> u64 { GRACE_WINDOW.with(|v| *v.borrow()) }
    }

    pub struct SweepFrequency;
    impl Get<u64> for SweepFrequency {
        fn get() -> u64 { SWEEP_FREQUENCY.with(|v| *v.borrow()) }
    }

    pub struct IssuanceFrequency;
    impl Get<u64> for IssuanceFrequency {
        fn get() -> u64 { ISSUANCE_FREQUENCY.with(|v| *v.borrow()) }
    }

    // Workaround for https://github.com/rust-lang/rust/issues/26925
    #[derive(Clone, Eq, PartialEq)]
    pub struct Runtime;
    parameter_types! {
        pub const BlockHashCount: BlockNumber = 250;
        pub const MaximumBlockWeight: Weight = 1_000_000_000;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
        pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    }
    impl system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
    }
    parameter_types! {
        pub const ExistentialDeposit: u64 = 0;
        pub const TransferFee: u64 = 0;
        pub const CreationFee: u64 = 0;
        pub const TransactionBaseFee: u64 = 0;
        pub const TransactionByteFee: u64 = 0;
    }
    impl balances::Trait for Runtime {
        type Balance = u64;
        type OnNewAccount = ();
        type OnFreeBalanceZero = ();
        type Event = ();
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type TransferFee = TransferFee;
        type CreationFee = CreationFee;
        type TransactionBaseFee = TransactionBaseFee;
        type TransactionByteFee = TransactionByteFee;
    }
    impl Trait for Runtime {
        type Currency = balances::Module<Test>;
        type Event = ();
        type ProposalBond = ProposalBond;
        type VoteBond = VoteBond;
        type ApplicationWindow = ApplicationWindow;
        type AbortWindow = AbortWindow;
        type VoteWindow = VoteWindow;
        type GraceWindow = GraceWindow;
        type SweepFrequency = SweepFrequency;
        type IssuanceFrequency = IssuanceFrequency;
    }

    pub struct ExtBuilder {
        proposal_bond: u64,
        vote_bond: u64,
        application_window: u64,
        abort_window: u64,
        vote_window: u64,
        grace_window: u64,
        sweep_frequency: u64,
        issuance_frequency: u64,
    } // can add more fields for scenarios like in `balances/mock`
    impl Default for ExtBuilder {
        fn default() -> Self {
            Self {
                proposal_bond: 2,
                vote_bond: 5,
                // 2 min / 6 seconds = 20
                application_window: 20,
                // 3 min / 6 seconds = 30
                abort_window:  30,
                // 5 min / 6 seconds = 50
                vote_window: 50,
                // 5 min / 6 seconds = 50
                grace_window: 50,
                // 10 min / 6 seconds = 100
                sweep_frequency: 100,
                issuance_frequency: 100,
            }
        }
    }
    impl ExtBuilder {
        pub fn proposal_bond(mut self, proposal_bond: u64) -> Self {
            self.proposal_bond = proposal_bond;
            self
        }
        pub fn vote_bond(mut self, vote_bond: u64) -> Self {
            self.vote_bond = vote_bond;
            self
        }
        pub fn application_window(mut self, application_window: u64) -> Self {
            self.application_window = application_window;
            self
        }
        pub fn abort_window(mut self, abort_window: u64) -> Self {
            self.abort_window = abort_window;
            self
        }
        pub fn vote_window(mut self, vote_window: u64) -> Self {
            self.vote_window = vote_window;
            self
        }
        pub fn grace_window(mut self, grace_window: u64) -> Self {
            self.grace_window = grace_window;
            self
        }
        pub fn sweep_frequency(mut self, sweep_frequency: u64) -> Self {
            self.sweep_frequency = sweep_frequency;
            self
        }
        pub fn issuance_frequency(mut self, issuance_frequency: u64) -> Self {
            self.issuance_frequency = issuance_frequency;
            self
        }
        pub fn set_associated_consts(&self) {
            PROPOSAL_BOND.with(|v| *v.borrow_mut() = self.proposal_bond);
            VOTE_BOND.with(|v| *v.borrow_mut() = self.vote_bond);
            APPLICATION_WINDOW.with(|v| *v.borrow_mut() = self.application_window);
            ABORT_WINDOW.with(|v| *v.borrow_mut() = self.abort_window);
            VOTE_WINDOW.with(|v| *v.borrow_mut() = self.vote_window);
            GRACE_WINDOW.with(|v| *v.borrow_mut() = self.grace_window);
            SWEEP_FREQUENCY.with(|v| *v.borrow_mut() = self.sweep_frequency);
            ISSUANCE_FREQUENCY.with(|v| *v.borrow_mut() = self.issuance_frequency);
	    }
        pub fn build(self) -> runtime_io::TestExternalities<Blake2Hasher> {
            self.set_associated_consts();
            let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap().0;
            t.extend(GenesisConfig::<Runtime> {
                balances: {
                    vec![
                        (1, 100),
                        (2, 50),
                        (3, 25),
                        (4, 10),
                        (5, 5),
                        (6, 1),
                    ]
                },
                members: {
                    vec![
                        (1, 10),
                        (2, 10),
                        (3, 50),
                        (4, 0),
                        (5, 0),
                        (6, 0),
                    ]
                },
            }.build_storage().unwrap().0);
            t.into()
        }
    }

    pub type DAO = Module<Runtime>;
    pub type Balances = balances::Module<Runtime>;

    // pub fn make_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    //     let mut t = system::GenesisConfig::default()
    //         .build_storage::<Test>()
    //         .unwrap()
    //         .0;
    //     t.extend(
    //         dao::GenesisConfig::<Test> {
    //             balances: vec![(0, 100), (1, 100), (2, 100), (3, 9), (4, 11)],
    //             members: vec![(0, 10), (1, 20), (2, 15), (3, 15), (4, 30)],
    //         }
    //         .build_storage()
    //         .unwrap()
    //         .0,
    //     );
    //     t.into()
    // }
}
