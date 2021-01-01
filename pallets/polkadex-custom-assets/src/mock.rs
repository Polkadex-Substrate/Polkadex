use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use frame_system as system;
use sp_core::{H256, Public, Pair};
use sp_runtime::{Perbill, testing::Header, traits::{BlakeTwo256, IdentityLookup}, MultiSignature};
use codec::Encode;
use sp_runtime::traits::{Hash, Verify, IdentifyAccount};
use crate::{Module, Trait, AssetCurrency, AssetIdProvider};
use super::*;
use sp_runtime::app_crypto::sr25519;


impl_outer_origin! {
	pub enum Origin for Test {}
}

// Configure a mock runtime to test the pallet.

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test2;
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
// parameter_types! {
//     pub const AssetId: T::Hash = H256::random();
// }
impl AssetIdProvider for Test {

    type AssetId = H256;


    fn asset_id() -> Self::AssetId {
        let asset_id: H256 = ("Native").using_encoded(<Test as frame_system::Trait>::Hashing::hash);
        asset_id
    }
}

parameter_types! {
pub const maxLocks: u32 = 10;
pub const existentialDeposit: u128 = 1;
}

impl Trait for Test {
    type Event = ();
    type Balance = u128;
    type MaxLocks = maxLocks;
    type ExistentialDeposit = existentialDeposit;
}

pub type PolkadexCustomAssetsModule = Module<Test>;
pub type Native = AssetCurrency<Test, Test>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    // system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let unit: FixedU128 = FixedU128::from(1000_000_000_000);
    let native_asset = H256::random();
    GenesisConfig::<Test>{
        initial_balance: unit,
        endowed_accounts: vec![0,1,2],
        assets: vec![native_asset],
        native_asset
    }.assimilate_storage(&mut t)
     .unwrap();

    t.into()
}