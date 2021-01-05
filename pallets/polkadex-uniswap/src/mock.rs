use crate::{Module, Config};
use sp_core::H256;
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill};
use frame_system as system;
use frame_system::limits::{BlockLength, BlockWeights};




impl_outer_origin! {
	pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const TradingPathLimit: usize = 10;
}



impl Config for Test {

    type Event = ();
    type TradingPathLimit = TradingPathLimit;
}

parameter_types! {
    pub const MaxSubAccounts: u32 = 10;
    pub const MaxRegistrars: u32 = 10;
}


impl pallet_idenity::Config for Test {

    type Event = ();
    type MaxSubAccounts = MaxSubAccounts;
    type MaxRegistrars= MaxRegistrars;


}
parameter_types! {
    pub const MaxLocks: u32 = 10;
    pub const ExistentialDeposit: u128 = 10;
}

impl polkadex_custom_assets::Config for Test{
    type Event = ();
    type Balance = u128;
    type MaxLocks = MaxLocks;
    type ExistentialDeposit = ExistentialDeposit;
}

pub type PolkadexSwapEngine = Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
