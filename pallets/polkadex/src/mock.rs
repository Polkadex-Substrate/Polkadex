use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use frame_system as system;
use pallet_generic_asset;
use sp_core::H256;
use sp_runtime::{Perbill, testing::Header, traits::{BlakeTwo256, IdentityLookup}};

use crate::{Module, Trait};

impl_outer_origin! {
	pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
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
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

parameter_types! {
pub const TradingPairReservationFee: u128 = 1_000_000_000_000;
}

impl Trait for Test {
    type Event = ();
    type TradingPairReservationFee = TradingPairReservationFee;
}

parameter_types! {
pub const MaxLocks: u32 = 10;
}

impl pallet_generic_asset::Trait for Test {
    type Balance = u128;
    type AssetId = u32;
    type Event = ();
    type MaxLocks = MaxLocks;
}

pub type DEXModule = Module<Test>;
type System = frame_system::Module<Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let endowed_accounts: Vec<u64> = vec![1, 2];
    const UNIT: u128 = 1_000_000_000_000;
    let mut genesis = system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_generic_asset::GenesisConfig::<Test> {
        assets: vec![0],
        initial_balance: 3 * UNIT,
        endowed_accounts: endowed_accounts
            .clone().into_iter().map(Into::into).collect(),
        next_asset_id: 1,
        staking_asset_id: 0,
        spending_asset_id: 0,
    }.assimilate_storage(&mut genesis).unwrap();
    let mut ext = sp_io::TestExternalities::new(genesis);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
