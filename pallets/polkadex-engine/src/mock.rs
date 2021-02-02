use frame_support::{impl_outer_origin, parameter_types, sp_io, weights::Weight};
use frame_system as system;
use frame_system::limits::{BlockLength, BlockWeights};
use sp_core::H256;
use sp_runtime::{Perbill, testing::Header, traits::{BlakeTwo256, IdentityLookup}};

use polkadex_custom_assets;
use polkadex_swap_engine::Event;

use crate::{Config, Module};

impl_outer_origin! {
	pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Config for Test {
    type BaseCallFilter = ();
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
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
}
const UNIT: u128 = 1_000_000_000_000;
parameter_types! {
pub const TradingPairReservationFee: u128 = 1*UNIT;
}

impl Config for Test {
    type Event = ();
    type TradingPairReservationFee = TradingPairReservationFee;
}


parameter_types! {
    pub const MaxSubAccounts: u32 = 10;
    pub const MaxRegistrars: u32 = 10;
}

impl pallet_idenity::Config for Test {
    type Event = ();
    type MaxSubAccounts = MaxSubAccounts;
    type MaxRegistrars= MaxRegistrars;
    type WeightInfo = ();
}


parameter_types! {
pub const MaxLocks: u32 = 10;
pub const ExistentialDeposit: u128 = 0;
}

impl polkadex_custom_assets::Config for Test {
    type Event = ();
    type Balance = u128;
    type MaxLocks = MaxLocks;
    type ExistentialDeposit = ExistentialDeposit;
}

parameter_types! {
pub const TradingPathLimit: usize = 6;
}

impl polkadex_swap_engine::Config for Test {
    type Event = ();
    type TradingPathLimit = TradingPathLimit;
}

pub type DEXModule = Module<Test>;
type System = frame_system::Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let endowed_accounts: Vec<u64> = vec![1, 2];
    const UNIT: u128 = 1_000_000_000_000;
    let mut genesis = system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let temp = H256::from_low_u64_be(8u64);
    let temp2 = H256::from_low_u64_be(10u64);
    polkadex_custom_assets::GenesisConfig::<Test> {
        native_asset: H256::zero(),
        assets: vec![H256::zero(), temp, temp2],
        initial_balance: DEXModule::convert_balance_to_fixed_u128(1000 * UNIT).unwrap(),
        endowed_accounts: endowed_accounts
            .clone().into_iter().map(Into::into).collect(),
    }.assimilate_storage(&mut genesis).unwrap();
    let mut ext = sp_io::TestExternalities::new(genesis);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
