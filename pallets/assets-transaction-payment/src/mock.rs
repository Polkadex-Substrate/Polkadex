use crate::*;
use frame_support::{
    parameter_types,
    traits::{fungibles::CreditOf,ConstU128, ConstU64, OnTimestampSet, OnUnbalanced, Currency},
    PalletId,
    weights::{ConstantMultiplier,WeightToFeePolynomial, WeightToFeeCoefficients, WeightToFeeCoefficient, constants::ExtrinsicBaseWeight}
};
use frame_system::EnsureRoot;
use polkadex_primitives::{Moment, Signature};
use sp_std::cell::RefCell;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup,ConvertInto},
    SaturatedConversion,
    FixedPointNumber
};
use sp_application_crypto::sp_core::H256;
use crate::payment::{HandleSwap, NegativeImbalanceOf};
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use polkadex_primitives::Balance;
use sp_runtime::Perquintill;
use smallvec::smallvec;
use sp_runtime::Perbill;

use crate::{self as assets_transaction_payment, Config};

pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;

pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckMortality<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    assets_transaction_payment::ChargeAssetTransactionPayment<Runtime>,
);


#[derive(PartialEq, Eq, Clone)]
pub struct UncheckedExtrinsic<Address, Call, Extra>
    where
        Extra: SignedExtension,
{
    /// The signature, address, number of extrinsics have come before from
    /// the same signer and an era describing the longevity of this transaction,
    /// if this is a signed extrinsic.
    pub signature: Option<(Address, Signature, Extra)>,
    /// The function that should be called.
    pub function: Call,
}

type UncheckedExtrinsic = UncheckedExtrinsic<Address, Call, SignedExtra>;
type Block = frame_system::mocking::MockBlock<Test>;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		AssetsTransactionPayment: assets_transaction_payment::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Call = Call;
    type Hashing = BlakeTwo256;
    type AccountId = sp_runtime::AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
}

pub struct WeightToFee;

impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        let p: Balance = 1_000_000_000_000;
        let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
        smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
    }
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Test {
    type Event = Event;
    type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate =
    TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

}
use polkadex_primitives::AccountId;
type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {

    }
}
parameter_types! {
	pub const LockPeriod: u64 = 201600;
	pub const MaxRelayers: u32 = 3;
}

parameter_types! {
	pub const AssetDeposit: u128 = 100;
	pub const ApprovalDeposit: u128 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u128 = 10;
	pub const MetadataDepositPerByte: u128 = 1;
}

impl pallet_assets::Config for Test {
    type Event = Event;
    type Balance = u128;
    type AssetId = u128;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<sp_runtime::AccountId32>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
}

thread_local! {
	pub static CAPTURED_MOMENT: RefCell<Option<Moment>> = RefCell::new(None);
}

pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
    fn on_timestamp_set(moment: Moment) {
        CAPTURED_MOMENT.with(|x| *x.borrow_mut() = Some(moment));
    }
}

impl pallet_timestamp::Config for Test {
    type Moment = Moment;
    type OnTimestampSet = MockOnTimestampSet;
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

pub struct AlternateTokenSwapper;
pub struct DealWithFees;

impl HandleSwap<Test> for AlternateTokenSwapper {
    fn swap(credit: CreditOf<AccountId, Assets>) -> NegativeImbalanceOf<Test> {
        NegativeImbalanceOf::new(credit.peek().saturated_into::<u128>().saturated_into())
    }
}


impl Config for Test {
    type Event = Event;
    type Fungibles = Assets;
    type OnChargeAssetTransaction = crate::payment::FungiblesAdapter<
        pallet_assets<::BalanceToAssetBalance<Balances, Test, ConvertInto>,
        AlternateTokenSwapper,
        DealWithFees,
    >;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    t.into()
}
