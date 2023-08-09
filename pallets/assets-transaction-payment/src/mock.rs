use crate::{
	pallet as xyz_transaction_payment,
	payment::{HandleSwap, NegativeImbalanceOf},
	*,
};
use frame_support::{
	parameter_types,
	traits::{
		fungibles::CreditOf, ConstU128, ConstU32, ConstU64, Currency, OnTimestampSet, OnUnbalanced,
	},
	weights::{
		ConstantMultiplier, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	},
};
use frame_system::EnsureRoot;
use pallet_transaction_payment::{CurrencyAdapter, Multiplier};
use polkadex_primitives::{AccountIndex, Balance, Moment};
use smallvec::smallvec;
use sp_application_crypto::sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	FixedPointNumber, Perbill, Perquintill, SaturatedConversion,
};
use sp_std::cell::RefCell;
use wallet_connector;

pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;

pub type SignedExtra = (
	frame_system::CheckSpecVersion<Test>,
	frame_system::CheckTxVersion<Test>,
	frame_system::CheckGenesis<Test>,
	frame_system::CheckMortality<Test>,
	frame_system::CheckNonce<Test>,
	frame_system::CheckWeight<Test>,
	ChargeAssetTransactionPayment<Test>,
);

pub type MockUncheckedExtrinsic =
	wallet_connector::unchecked_extrinsic::UncheckedExtrinsic<Address, Call, SignedExtra>;
pub type MockBlock = sp_runtime::generic::Block<Header, MockUncheckedExtrinsic>;

type Block = MockBlock;
type UncheckedExtrinsic = MockUncheckedExtrinsic;

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
		AssetsTransactionPayment: xyz_transaction_payment::{Pallet, Call, Storage, Event<T>},
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
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type Balance = u128;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ConstU128<10>;
	type AccountStore = System;
	type WeightInfo = ();
}

pub struct WeightToFee;

impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let result = smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_float(0.0),
			coeff_integer: 0,
		}];

		result
	}
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Test {
	type Event = Event;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
}
use polkadex_primitives::AccountId;
type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

parameter_types! {
	pub const LockPeriod: u64 = 201600;
	pub const MaxRelayers: u32 = 3;
}

parameter_types! {
	pub const AssetDeposit: u128 = 2;
	pub const ApprovalDeposit: u128 = 0;
	pub const StringLimit: u32 = 20;
	pub const MetadataDepositBase: u128 = 0;
	pub const MetadataDepositPerByte: u128 = 0;
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

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut _fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		//empty method
	}
}

pub struct AlternateTokenSwapper;
impl HandleSwap<Test> for AlternateTokenSwapper {
	fn swap(credit: CreditOf<AccountId, Assets>) -> NegativeImbalanceOf<Test> {
		NegativeImbalanceOf::new(credit.peek().saturated_into::<u128>().saturated_into())
	}
}

impl xyz_transaction_payment::Config for Test {
	type Event = Event;
	type Fungibles = Assets;
	type OnChargeAssetTransaction = payment::FungiblesAdapter<
		pallet_assets::BalanceToAssetBalance<Balances, Test, ConvertInto>,
		AlternateTokenSwapper,
		DealWithFees,
	>;
	type GovernanceOrigin = EnsureRoot<sp_runtime::AccountId32>;
	type WeightInfo = crate::weights::WeightInfo<Test>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}
