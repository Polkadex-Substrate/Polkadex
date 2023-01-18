use crate::pallet as thea_staking;
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
	PalletId,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type Balance = u128;
use crate::*;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		ChainBridge: chainbridge::{Pallet, Storage, Call, Event<T>},
		AssetHandler: asset_handler::pallet::{Pallet, Storage, Call, Event<T>},
		Thea: thea::pallet::{Pallet, Call, Storage, Event<T>},
		TheaStaking: thea_staking::{Pallet, Call, Storage, Event<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const SessionLength: u64 = 10;
	pub const UnbondingDelay: u32 = 10;
	pub const MaxUnlockChunks: u32 = 10;
	pub const CandidateBond: Balance = 1000_000_000_000;
	pub const StakingReserveIdentifier: [u8; 8] = [1u8;8];
	pub const StakingDataPruneDelay: u32 = 6;
}

impl thea_staking::Config for Test {
	type Event = Event;
	type SessionLength = SessionLength;
	type UnbondingDelay = UnbondingDelay;
	type MaxUnlockChunks = MaxUnlockChunks;
	type CandidateBond = CandidateBond;
	type StakingReserveIdentifier = StakingReserveIdentifier;
	type StakingDataPruneDelay = StakingDataPruneDelay;
	type SessionChangeNotifier = Thea;
}

parameter_types! {
	pub const ChainId: u8 = 1;
	pub const ProposalLifetime: u64 = 1000;
	pub const ChainbridgePalletId: PalletId = PalletId(*b"CSBRIDGE");
}

impl chainbridge::Config for Test {
	type Event = Event;
	type BridgeCommitteeOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type Proposal = Call;
	type BridgeChainId = ChainId;
	type ProposalLifetime = ProposalLifetime;
}

impl asset_handler::pallet::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AssetManager = Assets;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type TreasuryPalletId = ChainbridgePalletId;
	type WeightInfo = asset_handler::weights::WeightInfo<Test>;
}

parameter_types! {
	pub const TheaPalletId: PalletId = PalletId(*b"THBRIDGE");
}

impl thea::pallet::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type TheaPalletId = TheaPalletId;
}

//defined trait for Session Change
impl<Test> SessionChanged for thea::pallet::Pallet<Test> {
	type Network = Network;
	type BLSPublicKey = BLSPublicKey;
	type AccountId = u64;
	type OnSessionChange = OnSessionChange<u64>;
	fn on_new_session(
		map: BTreeMap<Self::Network, Self::OnSessionChange>,
	) {
	}
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type AssetId = u128;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureSigned<Self::AccountId>;
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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
