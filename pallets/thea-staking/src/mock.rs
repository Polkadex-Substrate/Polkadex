use crate::pallet as thea_staking;
use frame_election_provider_support::{onchain, ElectionDataProvider, SequentialPhragmen};
use frame_support::{
	parameter_types,
	traits::{
		ConstU16, ConstU64, Currency, EitherOfDiverse, KeyOwnerProofSystem, OnUnbalanced,
		U128CurrencyToVote,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
		ConstantMultiplier, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	PalletId,
};
use frame_system as system;
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pallet_session::historical as pallet_session_historical;
use pallet_transaction_payment::*;
use polkadex_primitives::{AccountId, Moment};
use smallvec::smallvec;
use sp_core::{crypto::KeyTypeId, H256};
use sp_runtime::{
	curve::PiecewiseLinear,
	impl_opaque_keys,
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, OpaqueKeys},
	Perquintill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type Balance = u128;
use crate::*;

const PDEX: Balance = 1_000_000_000_000;
const UNITS: Balance = PDEX;
const DOLLARS: Balance = PDEX; // 1_000_000_000_000
const CENTS: Balance = DOLLARS / 100; // 10_000_000_000
const MILLICENTS: Balance = CENTS / 1_000; // 1000_000_000
const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
}

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
		Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned},
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		Historical: pallet_session_historical::{Pallet},
		Offences: pallet_offences::{Pallet, Storage, Event},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} ,
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
	type AccountId = Self::AccountId;
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
	type ValidatorSet = Historical;
	type ReportMisbehavior = Offences;
}

parameter_types! {
	pub const ChainId: u8 = 1;
	pub const ProposalLifetime: u64 = 1000;
	pub const ChainbridgePalletId: PalletId = PalletId(*b"CSBRIDGE");
	pub const ParachainNetworkId: u8 = 1;
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
	type ParachainNetworkId = ParachainNetworkId;
}

parameter_types! {
	pub const TheaPalletId: PalletId = PalletId(*b"THBRIDGE");
	pub const WithdrawalSize: u32 = 10;
}

impl thea::pallet::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type TheaPalletId = TheaPalletId;
	type WithdrawalSize = WithdrawalSize;
}

//defined trait for Session Change
impl<Test> SessionChanged for thea::pallet::Pallet<Test> {
	type Network = Network;
	type OnSessionChange = OnSessionChange<u64>;
	fn on_new_session(map: BTreeMap<Self::Network, Self::OnSessionChange>) {}
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

impl pallet_session::Config for Test {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Test>;
}

impl pallet_offences::Config for Test {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = pallet_staking::Exposure<Self::AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Test>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = 12000 / 2;
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Test>;
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

impl pallet_election_provider_multi_phase::MinerConfig for Test {
	type AccountId = AccountId;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
		<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
		as
		pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

struct OnChainSeqPhragmen;

impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Test>,
	>;
	type DataProvider = <Test as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Test>;
}

impl onchain::BoundedConfig for OnChainSeqPhragmen {
	type VotersBound = MaxElectingVoters;
	type TargetsBound = ConstU32<2_000>;
}

type CouncilCollective = pallet_collective::Instance1;

struct StakingBenchmarkingConfig;

impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxValidators = ConstU32<1000>;
	type MaxNominators = ConstU32<1000>;
}

impl pallet_staking::Config for Test {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
	type MaxNominations = ();
	type RewardRemainder = ();
	type Event = Event;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = EitherOfDiverse<
		EnsureRoot<Self::AccountId>,
		pallet_collective::EnsureProportionAtLeast<Self::AccountId, CouncilCollective, 3, 4>,
	>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Test>;
	type MaxUnlockingChunks = ConstU32<32>;
	type OnStakerSlash = ();
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Test>;
}

parameter_types! {
	// Six session in a an era (24 hrs)
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	// 28 era for unbonding (28 days)
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = 6000 as u64;
	pub const ExpectedBlockTime: Moment = 12000;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
	pub const MaxAuthorities: u32 = 200;
}

impl pallet_babe::Config for Test {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;

	type KeyOwnerProofSystem = Historical;

	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		pallet_babe::AuthorityId,
	)>>::IdentificationTuple;

	type HandleEquivocation =
		pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;
	type WeightInfo = ();
	type DisabledValidators = Session;
	type MaxAuthorities = MaxAuthorities;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		// Before, we launch the products we want 50% of supply to be staked
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 4 seconds of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 4 * WEIGHT_PER_SECOND;

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = 12000 / 4;
	pub const UnsignedPhase: u32 = 12000 / 4 ;
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	// 40 PDEXs fixed deposit..
	pub const SignedDepositBase: Balance = deposit(2, 0);
	// 0.01 PDEX per KB of solution data.
	pub const SignedDepositByte: Balance = deposit(0, 10) / 1024;
	// Each good submission will get 1 DOT as reward
	pub SignedRewardBase: Balance = UNITS;
	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(1u32, 10_000);
	pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
		.saturating_sub(BlockExecutionWeight::get());
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	// Solution can occupy 90% of normal block size
	pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);

	// miner configs
	pub const MinerMaxIterations: u32 = 10;
	pub OffchainRepeat: polkadex_primitives::BlockNumber = 5;
	pub MaxElectingVoters: u32 = 10_000;
	const OperationalFeeMultiplier: u8 = 5;
	const TransactionByteFee: Balance = 10 * MILLICENTS;
	const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	const AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	const MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

pub struct ElectionProviderBenchmarkConfig;

impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

impl pallet_election_provider_multi_phase::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type UnsignedPhase = UnsignedPhase;
	type SignedPhase = SignedPhase;
	type BetterSignedThreshold = ();
	type BetterUnsignedThreshold = BetterUnsignedThreshold;
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedMaxWeight = MinerMaxWeight;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase = SignedDepositBase;
	type SignedDepositByte = SignedDepositByte;
	type SignedDepositWeight = ();
	type MaxElectingVoters = MaxElectingVoters;
	type MaxElectableTargets = ConstU16<{ u16::MAX }>;
	type SlashHandler = ();
	// burn slashes
	type RewardHandler = ();
	type DataProvider = Staking;
	type Fallback = onchain::BoundedExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::BoundedExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
	>;
	type ForceOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	>;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Test>;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

struct DealWithFees;

struct WeightToFeeMock;

impl WeightToFeePolynomial for WeightToFeeMock {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		let p = CENTS;
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(mut fees) = fees_then_tips.next() {
			if let Some(tips) = fees_then_tips.next() {
				tips.merge_into(&mut fees);
			}
			// Sent everything to treasury
			//Treasury::on_unbalanced(fees);
		}
	}
}

impl pallet_transaction_payment::Config for Test {
	type Event = Event;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFeeMock;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate =
		TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
	}
}
