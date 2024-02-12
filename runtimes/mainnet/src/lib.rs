// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
#![deny(unused_crate_dependencies)]

use constants::{currency::*, time::*};
use frame_election_provider_support::{
	bounds::ElectionBoundsBuilder, onchain, ElectionDataProvider, SequentialPhragmen,
};
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	pallet_prelude::{ConstU32, RuntimeDebug},
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, Currency, EitherOfDiverse, EnsureOrigin, EqualPrivilegeOnly,
		Everything, Get, Imbalance, InstanceFilter, KeyOwnerProofSystem, LockIdentifier,
		OnUnbalanced,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	PalletId,
};
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned, RawOrigin,
};

#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
pub use polkadex_primitives::{
	AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature,
};
use polkadex_primitives::{AssetId, Nonce};
use rust_decimal::Decimal;
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_inherents::{CheckInherentsResult, InherentData};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		self, AccountIdConversion, BlakeTwo256, Block as BlockT, BlockNumberProvider, NumberFor,
		OpaqueKeys, SaturatedConversion, StaticLookup,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
};
use sp_std::{prelude::*, vec};
use sp_storage as _;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;

/// Constant values used within the runtime.
pub mod constants;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is \
						built with `SKIP_WASM_BUILD` flag and it is only usable for \
						production chains. Please rebuild with the flag disabled.",
	)
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node"),
	impl_name: create_runtime_str!("polkadex-official"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 328,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	state_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(mut fees) = fees_then_tips.next() {
			if let Some(tips) = fees_then_tips.next() {
				tips.merge_into(&mut fees);
			}
			// Sent everything to treasury
			Treasury::on_unbalanced(fees);
		}
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 4 seconds of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(4), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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
	pub const SS58Prefix: u8 = 88;
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = Indices;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = Nonce;
	type Block = Block;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}
use scale_info::TypeInfo;
use sp_core::crypto::AccountId32;
use sp_npos_elections::ExtendedBalance;

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
#[allow(clippy::unnecessary_cast)]
pub enum ProxyType {
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	Staking = 3,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => false,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..)
					| RuntimeCall::Indices(pallet_indices::Call::transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Council(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::Elections(..)
					| RuntimeCall::Treasury(..)
					| RuntimeCall::OrderbookCommittee(..)
			),
			ProxyType::Staking => matches!(c, RuntimeCall::Staking(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	// Retry a scheduled item every 10 blocks (1 minute) until the preimage exists.
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type Preimages = Preimage;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = DOLLARS;
	// One cent: PDEX 10,000 / MB
	pub const PreimageByteDeposit: Balance = CENTS;
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS as u64;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
	pub const MaxAuthorities: u32 = 200;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const IndexDeposit: Balance = PDEX;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = PDEX;
	pub const MaxLocks: u32 = 50;
	pub const MaxHolds: u32 = 50;
	pub const MaxFreezes: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type MaxHolds = MaxHolds;
	type MaxFreezes = MaxFreezes;
}
use sp_runtime::traits::{Bounded, ConvertInto};
parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(5, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
		pub orderbook: OCEX,
		pub thea: Thea,
	}
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
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

parameter_types! {
	// Six session in a an era (24 hrs)
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	// 28 era for unbonding (28 days)
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	pub HistoryDepth: u32 = 84;
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxValidators = ConstU32<1000>;
	type MaxNominators = ConstU32<1000>;
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds = ElectionBounds;
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
	type HistoryDepth = HistoryDepth;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Runtime>;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type EventListeners = ();
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4;
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_SLOTS / 4 ;

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
	// Solution can occupy 90% of normal block size
	pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);

	// miner configs
	pub const MinerMaxIterations: u32 = 10;
	pub OffchainRepeat: BlockNumber = 5;
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
// pub const MAX_NOMINATIONS: u32 =
//     <NposCompactSolution16 as sp_npos_elections::NposSolution>::LIMIT as u32;

parameter_types! {
	pub const MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	pub MaxElectingVoters: u32 = 40_000;
	pub MaxElectableTargets: u16 = 10_000;
	// OnChain values are lower.
	pub MaxOnChainElectingVoters: u32 = 5000;
	pub MaxOnChainElectableTargets: u16 = 1250;
	// The maximum winners that can be elected by the Election pallet which is equivalent to the
	// maximum active validators the staking pallet can have.
	pub MaxActiveValidators: u32 = 1000;
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
}

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
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

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<(usize, ExtendedBalance)>> for OffchainRandomBalancing {
	fn get() -> Option<(usize, ExtendedBalance)> {
		use sp_runtime::traits::TrailingZeroInput;
		let iters = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		Some((iters, 0))
	}
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type MaxWinners = MaxActiveValidators;

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
impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
	type MaxWinners = MaxActiveValidators;
	type ElectionBounds = ElectionBounds;
	type SlashHandler = ();
	// burn slashes
	type RewardHandler = ();
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
	>;
	type ForceOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	>;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const CandidacyBond: Balance = 100 * PDEX;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 5;
	pub const DesiredRunnersUp: u32 = 5;
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
	pub const MaxCandidates: u32 = 1000;
	pub const MaxVoters: u32 = 10*1000;
	pub const MaxVotesPerVoter: u32 = 16;
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ElectionsPhragmenPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type MaxCandidates = MaxCandidates;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 7 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRootOrHalfCouncil;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const OrderbookMotionDuration: BlockNumber = DAYS;
	pub const OrderbookMaxProposals: u32 = 100;
	pub const OrderbookMaxMembers: u32 = 3;
}

type OrderbookCollective = pallet_collective::Instance3;
impl pallet_collective::Config<OrderbookCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = OrderbookMotionDuration;
	type MaxProposals = OrderbookMaxProposals;
	type MaxMembers = OrderbookMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, TechnicalCollective, 1, 2>,
	>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

type EnsureRootOrHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

type EnsureRootOrHalfOrderbookCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, OrderbookCollective, 1, 2>,
>;

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrHalfCouncil;
	type RemoveOrigin = EnsureRootOrHalfCouncil;
	type SwapOrigin = EnsureRootOrHalfCouncil;
	type ResetOrigin = EnsureRootOrHalfCouncil;
	type PrimeOrigin = EnsureRootOrHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * PDEX;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = PDEX;
	pub const DataDepositPerByte: Balance = CENTS;
	pub const BountyDepositBase: Balance = PDEX;
	pub const BountyDepositPayoutDelay: BlockNumber = 8 * DAYS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 10 * PDEX;
	pub const MaxApprovals: u32 = 100;
	pub const MaxActiveChildBountyCount: u32 = 5;
	pub const ChildBountyValueMinimum: Balance = PDEX;
	pub const CuratorDepositMax: Balance = 100 * PDEX;
	pub const CuratorDepositMin: Balance = 10 * PDEX;
	pub const ChildBountyCuratorDepositBase: Permill = Permill::from_percent(10);
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ChildBounties;
	type CuratorDepositMultiplier = BountyCuratorDeposit;
	type CuratorDepositMax = CuratorDepositMax;
	type CuratorDepositMin = CuratorDepositMin;
}

impl pallet_child_bounties::Config for Runtime {
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
	>;
	type RejectOrigin = EnsureRootOrHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type ProposalBondMaximum = ();
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
}

parameter_types! {
	// When proposals are moved to public voting
	pub const LaunchPeriod: BlockNumber = 15 * DAYS;
	// How long voting should last
	pub const VotingPeriod: BlockNumber = 15 * DAYS;
	// Fast track voting for techincal council
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	// Minimum deposit for creating a proposal
	pub MinimumDeposit: Balance = 100 * PDEX;
	// Time between approved proposals are executed on-chain
	// EnactmentPeriod > unbonding period of staking
	pub const EnactmentPeriod: BlockNumber = 30 * DAYS;
	// Minimum period of vote locking
	// Note: VoteLockingPeriod >= EnactmentPeriod
	pub const VoteLockingPeriod: BlockNumber = 30 * DAYS;
	// Cool-off period before a vetoed proposal can be submitted back again
	pub const CooloffPeriod: BlockNumber = 28 * DAYS;
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;

}

impl pallet_democracy::Config for Runtime {
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Preimages = Preimage;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MinimumDeposit = MinimumDeposit;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type CooloffPeriod = CooloffPeriod;
	type MaxVotes = MaxVotes;
	type MaxProposals = MaxProposals;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
	>;
	/// A majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
	>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
	>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>,
	>;
	type InstantOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	// Any single technical committee member or root origin may veto a coming council proposal,
	// however they can only do it once and it lasts only for the cooloff period.
	// NOTE: Technical Council cannot be greater than MAX_VETOERS
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type PalletsOrigin = OriginCaller;
	type Slash = Treasury;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	/// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(tip, None),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}
parameter_types! {
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	pub const AssetAccountDeposit: Balance = DOLLARS;
	pub const ApprovalDeposit: Balance = DOLLARS;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	pub const MetadataDepositPerByte: Balance = DOLLARS;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ();
	type AssetId = u128;
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetU128;
}
#[cfg(feature = "runtime-benchmarks")]
pub struct AssetU128;
#[cfg(feature = "runtime-benchmarks")]
use pallet_assets::BenchmarkHelper;
use sp_staking::currency_to_vote::U128CurrencyToVote;

#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<parity_scale_codec::Compact<u128>> for AssetU128 {
	fn create_asset_id_parameter(id: u32) -> parity_scale_codec::Compact<u128> {
		parity_scale_codec::Compact::from(id as u128)
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_asset_conversion::BenchmarkHelper<u128, AssetId> for AssetU128 {
	fn asset_id(id: u32) -> u128 {
		id as u128
	}

	fn multiasset_id(id: u32) -> AssetId {
		AssetId::Asset(id as u128)
	}
}

parameter_types! {
	pub const BasicDeposit: Balance = deposit(1,258);       // 258 bytes on-chain
	pub const FieldDeposit: Balance = deposit(0,66);        // 66 bytes on-chain
	pub const SubAccountDeposit: Balance = deposit(1,53);   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type RegistrarOrigin = EnsureRootOrHalfCouncil;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * PDEX;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 5 * PDEX;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_recovery::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub MinVestedTransfer: Balance = PDEX;
	pub const MaxVestingSchedules: u32 = 300;
}

pub struct SusbtrateBlockNumberProvider;
impl BlockNumberProvider for SusbtrateBlockNumberProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		System::block_number()
	}
}

pub struct EnsureRootOrTreasury;
impl EnsureOrigin<RuntimeOrigin> for EnsureRootOrTreasury {
	type Success = AccountId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Into::<Result<RawOrigin<AccountId>, RuntimeOrigin>>::into(o).and_then(|o| match o {
			RawOrigin::Root => Ok(TreasuryPalletId::get().into_account_truncating()),
			RawOrigin::Signed(caller) => {
				if caller == TreasuryPalletId::get().into_account_truncating() {
					Ok(caller)
				} else {
					Err(RuntimeOrigin::from(Some(caller)))
				}
			},
			r => Err(RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::from(RawOrigin::Signed(AccountId::new([0u8; 32]))))
	}
}

impl orml_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureRootOrTreasury;
	type WeightInfo = ();
	type MaxVestingSchedules = MaxVestingSchedules;
	type BlockNumberProvider = SusbtrateBlockNumberProvider;
}
parameter_types! {
	pub const LockPeriod: BlockNumber = 201600;
	pub const MaxRelayers: u32 = 3;
}

impl pdex_migration::pallet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxRelayers = MaxRelayers;
	type LockPeriod = LockPeriod;
}

parameter_types! {
	pub const PolkadexTreasuryModuleId: PalletId = PalletId(*b"polka/tr");
	pub TreasuryModuleAccount: AccountId = PolkadexTreasuryModuleId::get().into_account_truncating();
}

//impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const ProxyLimit: u32 = 3;
	pub const OcexPalletId: PalletId = PalletId(*b"OCEX_LMP");
	pub const MsPerDay: u64 = 86_400_000;
}

impl pallet_ocex_lmp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = OcexPalletId;
	type NativeCurrency = Balances;
	type OtherAssets = Assets;
	type EnclaveOrigin = EnsureSigned<AccountId>;
	type AuthorityId = pallet_ocex_lmp::sr25519::AuthorityId;
	type GovernanceOrigin = EnsureRootOrHalfCouncil;
	type WeightInfo = pallet_ocex_lmp::weights::WeightInfo<Runtime>;
}

//Install rewards Pallet
parameter_types! {
	pub const RewardsPalletId: PalletId = PalletId(*b"REWARDSQ");
}

impl pallet_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = RewardsPalletId;
	type NativeCurrency = Balances;
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
	type GovernanceOrigin = EnsureRootOrHalfCouncil;
	type WeightInfo = pallet_rewards::weights::WeightInfo<Runtime>;
}

parameter_types! {
	pub const LiquidityPalletId: PalletId = PalletId(*b"LIQU/IDI");
}

impl liquidity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LiquidityPalletId;
	type NativeCurrency = Balances;
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
	type GovernanceOrigin = EnsureRootOrHalfOrderbookCouncil;
	type CallOcex = OCEX;
	type WeightInfo = liquidity::weights::WeightInfo<Runtime>;
}

use polkadex_primitives::POLKADEX_NATIVE_ASSET_ID;

parameter_types! {
	pub const PolkadexAssetId: u128 = POLKADEX_NATIVE_ASSET_ID;
	pub const PDEXHolderAccount: AccountId32 = AccountId32::new([1u8;32]); //TODO Chnage Holder Account
}

impl thea::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TheaId = thea::ecdsa::AuthorityId;
	type Signature = thea::ecdsa::AuthoritySignature;
	type MaxAuthorities = MaxAuthorities;
	type Executor = TheaExecutor;
	type Currency = Balances;
	type GovernanceOrigin = EnsureRootOrHalfCouncil;
	type WeightInfo = thea::weights::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TheaPalletAccount: PalletId = PalletId(*b"th/accnt");
	pub const WithdrawalSize: u32 = 10;
	pub const ParaId: u32 = 2040;
}

impl thea_executor::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Assets = Assets;
	type AssetId = u128;
	type AssetCreateUpdateOrigin = EnsureRootOrHalfCouncil;
	type Executor = Thea;
	type NativeAssetId = PolkadexAssetId;
	type TheaPalletId = TheaPalletAccount;
	type WithdrawalSize = WithdrawalSize;
	type ParaId = ParaId;
	type TheaExecWeightInfo = thea_executor::weights::WeightInfo<Runtime>;
	type Swap = AssetConversion;
	type MultiAssetIdAdapter = AssetId;
	type AssetBalanceAdapter = u128;
	type GovernanceOrigin = EnsureRootOrHalfCouncil;
	type ExistentialDeposit = ExistentialDeposit;
}

#[cfg(feature = "runtime-benchmarks")]
impl thea_message_handler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type TheaId = thea::ecdsa::AuthorityId;
	type Signature = thea::ecdsa::AuthoritySignature;
	type MaxAuthorities = MaxAuthorities;
	type Executor = TheaExecutor;
	type WeightInfo = thea_message_handler::weights::WeightInfo<Runtime>;
}
use frame_support::ord_parameter_types;
ord_parameter_types! {
	pub const AssetConversionOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&AssetConversionPalletId::get());
}

impl pallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = Assets;
	type OnChargeAssetTransaction =
		pallet_asset_conversion_tx_payment::AssetConversionAdapter<Balances, AssetConversion>;
}

impl pallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = Assets;
	type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
		pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto>,
		CreditToBlockAuthor,
	>;
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub AllowMultiAssetPools: bool = true;
	pub const PoolSetupFee: Balance = DOLLARS; // should be more or equal to the existential deposit
	pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency has 10-12 decimals.
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);  // should be non-zero if AllowMultiAssetPools is true, otherwise can be zero.
}

impl pallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Balance = u128;
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type HigherPrecisionBalance = u128;
	type AssetId = u128;
	type MultiAssetId = AssetId;
	type MultiAssetIdConverter = polkadex_primitives::AssetIdConverter;
	type PoolAssetId = u128;
	type Assets = Assets;
	type PoolAssets = Assets;
	type LPFee = ConstU32<3>; // means 0.3%
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeReceiver = AssetConversionOrigin;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type MintMinLiquidity = MintMinLiquidity;
	type MaxSwapPathLength = ConstU32<4>;
	type PalletId = AssetConversionPalletId;
	type AllowMultiAssetPools = AllowMultiAssetPools;
	type WeightInfo = pallet_asset_conversion::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetU128;
}

parameter_types! {
	pub StatementCost: Balance = DOLLARS;
	pub StatementByteCost: Balance = 100 * MILLICENTS;
	pub const MinAllowedStatements: u32 = 4;
	pub const MaxAllowedStatements: u32 = 10;
	pub const MinAllowedBytes: u32 = 1024;
	pub const MaxAllowedBytes: u32 = 4096;
}

impl pallet_statement::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type StatementCost = StatementCost;
	type ByteCost = StatementByteCost;
	type MinAllowedStatements = MinAllowedStatements;
	type MaxAllowedStatements = MaxAllowedStatements;
	type MinAllowedBytes = MinAllowedBytes;
	type MaxAllowedBytes = MaxAllowedBytes;
}

#[cfg(feature = "runtime-benchmarks")]
construct_runtime!(
	pub struct Runtime
	{
		System: frame_system = 0,
		Utility: pallet_utility = 1,
		Babe: pallet_babe = 2,
		Timestamp: pallet_timestamp = 3,
		Authorship: pallet_authorship = 4,
		Indices: pallet_indices = 5,
		Balances: pallet_balances = 6,
		TransactionPayment: pallet_transaction_payment = 7,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 8,
		Staking: pallet_staking = 9,
		Session: pallet_session = 10,
		Council: pallet_collective::<Instance1> = 11,
		TechnicalCommittee: pallet_collective::<Instance2> = 12,
		Elections: pallet_elections_phragmen = 13,
		TechnicalMembership: pallet_membership::<Instance1> = 14,
		Grandpa: pallet_grandpa = 15,
		Treasury: pallet_treasury = 16,
		Sudo: pallet_sudo = 17,
		ImOnline: pallet_im_online = 18,
		AuthorityDiscovery: pallet_authority_discovery = 19,
		Offences: pallet_offences = 20,
		Historical: pallet_session_historical = 21,
		Identity: pallet_identity = 22,
		Recovery: pallet_recovery = 23,
		Scheduler: pallet_scheduler = 24,
		Proxy: pallet_proxy = 25,
		Multisig: pallet_multisig = 26,
		Bounties: pallet_bounties = 27,
		OrmlVesting: orml_vesting = 28,
		PDEXMigration: pdex_migration::pallet = 29,
		Democracy: pallet_democracy = 30,
		Preimage: pallet_preimage = 31,
		//RandomnessCollectiveFlip: pallet_randomness_collective_flip = 32,
		ChildBounties: pallet_child_bounties = 33,
		Assets: pallet_assets = 34,
		OCEX: pallet_ocex_lmp = 35,
		OrderbookCommittee: pallet_collective::<Instance3> = 36,
		Thea: thea::pallet = 39,
		Rewards: pallet_rewards = 40,
		Liquidity: liquidity = 41,
		TheaExecutor: thea_executor::pallet = 44,
		TheaMH: thea_message_handler::pallet = 45,
		AssetConversion: pallet_asset_conversion = 46,
		AssetConversionTxPayment: pallet_asset_conversion_tx_payment = 47,
		Statement: pallet_statement = 48,
		AssetTxPayment: pallet_asset_tx_payment = 49,
	}
);

#[cfg(not(feature = "runtime-benchmarks"))]
construct_runtime!(
	pub struct Runtime {
		System: frame_system = 0,
		Utility: pallet_utility = 1,
		Babe: pallet_babe = 2,
		Timestamp: pallet_timestamp = 3,
		Authorship: pallet_authorship = 4,
		Indices: pallet_indices = 5,
		Balances: pallet_balances = 6,
		TransactionPayment: pallet_transaction_payment = 7,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 8,
		Staking: pallet_staking = 9,
		Session: pallet_session = 10,
		Council: pallet_collective::<Instance1> = 11,
		TechnicalCommittee: pallet_collective::<Instance2> = 12,
		Elections: pallet_elections_phragmen = 13,
		TechnicalMembership: pallet_membership::<Instance1> = 14,
		Grandpa: pallet_grandpa = 15,
		Treasury: pallet_treasury = 16,
		Sudo: pallet_sudo = 17,
		ImOnline: pallet_im_online = 18,
		AuthorityDiscovery: pallet_authority_discovery = 19,
		Offences: pallet_offences = 20,
		Historical: pallet_session_historical = 21,
		Identity: pallet_identity = 22,
		Recovery: pallet_recovery = 23,
		Scheduler: pallet_scheduler = 24,
		Proxy: pallet_proxy = 25,
		Multisig: pallet_multisig = 26,
		Bounties: pallet_bounties = 27,
		OrmlVesting: orml_vesting = 28,
		PDEXMigration: pdex_migration::pallet = 29,
		Democracy: pallet_democracy = 30,
		Preimage: pallet_preimage = 31,
		//RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 32,
		ChildBounties: pallet_child_bounties = 33,
		Assets: pallet_assets = 34,
		OCEX: pallet_ocex_lmp = 35,
		OrderbookCommittee: pallet_collective::<Instance3> = 36,
		Thea: thea::pallet = 39,
		Rewards: pallet_rewards = 40,
		Liquidity: liquidity = 41,
		TheaExecutor: thea_executor::pallet = 44,
		AssetConversion: pallet_asset_conversion = 46,
		AssetConversionTxPayment: pallet_asset_conversion_tx_payment = 47,
		Statement: pallet_statement = 48,
		AssetTxPayment: pallet_asset_tx_payment = 49,
	}
);
/// Digest item type.
pub type DigestItem = generic::DigestItem;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in
/// `bin/polkadex-node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

use crate::{
	impls::CreditToBlockAuthor,
	sp_api_hidden_includes_construct_runtime::hidden_include::traits::fungible::Inspect,
};
use orderbook_primitives::ObCheckpointRaw;
impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl pallet_asset_conversion::AssetConversionApi<
		Block,
		Balance,
		u128,
		AssetId
	> for Runtime
	{
		fn quote_price_exact_tokens_for_tokens(asset1: AssetId, asset2: AssetId, amount: u128, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee)
		}

		fn quote_price_tokens_for_exact_tokens(asset1: AssetId, asset2: AssetId, amount: u128, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee)
		}

		fn get_reserves(asset1: AssetId, asset2: AssetId) -> Option<(Balance, Balance)> {
			AssetConversion::get_reserves(&asset1, &asset2).ok()
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl rpc_assets_runtime_api::PolkadexAssetHandlerRuntimeApi<Block,AccountId,Hash> for Runtime {
		fn account_balances(assets : Vec<u128>, account_id : AccountId) ->  Vec<u128> {
			assets
				.iter()
				.map(|asset| {
					if *asset != POLKADEX_NATIVE_ASSET_ID {
					Assets::balance(*asset, &account_id).saturated_into()
					}else{
					Balances::balance(&account_id).saturated_into()
				}
				})
				.collect()
		}
	}

	impl pallet_rewards_runtime_api::PolkadexRewardsRuntimeApi<Block, AccountId, Hash> for Runtime {
		fn account_info(account_id : AccountId, reward_id: u32) ->  Result<polkadex_primitives::rewards::RewardsInfoByAccount<u128>, DispatchError> {
			Rewards::account_info(account_id, reward_id)
		}
	}

	impl pallet_ocex_runtime_api::PolkadexOcexRuntimeApi<Block, AccountId, Hash> for Runtime {
		fn get_ob_recover_state() ->  Result<Vec<u8>, DispatchError> { Ok(OCEX::get_ob_recover_state()?.encode()) }
		fn get_balance(from: AccountId, of: AssetId) -> Result<Decimal, DispatchError> { OCEX::get_balance(from, of) }
		fn fetch_checkpoint() -> Result<ObCheckpointRaw, DispatchError> {
			OCEX::fetch_checkpoint()
		}
		fn calculate_inventory_deviation() -> Result<sp_std::collections::btree_map::BTreeMap<AssetId,Decimal>,
		DispatchError> {
			OCEX::calculate_inventory_deviation()
		}
	}

	impl sp_statement_store::runtime_api::ValidateStatement<Block> for Runtime {
		fn validate_statement(
			source: sp_statement_store::runtime_api::StatementSource,
			statement: sp_statement_store::Statement,
		) -> Result<sp_statement_store::runtime_api::ValidStatement, sp_statement_store::runtime_api::InvalidStatement> {
			Statement::validate_statement(source, statement)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;
			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: BABE_GENESIS_EPOCH_CONFIG.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;
			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}


	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}


	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmark!(list, extra, pallet_ocex_lmp, OCEX);
			list_benchmark!(list, extra, pdex_migration, PDEXMigration);
			list_benchmark!(list, extra, pallet_rewards, Rewards);
			list_benchmark!(list, extra, liquidity, Liquidity);
			list_benchmark!(list, extra, thea_executor, TheaExecutor);
			list_benchmark!(list, extra, thea, Thea);
			list_benchmark!(list, extra, thea_message_handler, TheaMH);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark};
			use sp_storage::TrackedStorageKey;
			impl frame_system_benchmarking::Config for Runtime {}

			let allowlist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
				// Treasury Account
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &allowlist);

			add_benchmark!(params, batches, pallet_ocex_lmp, OCEX);
			add_benchmark!(params, batches, pdex_migration, PDEXMigration);
			add_benchmark!(params, batches, pallet_rewards, Rewards);
			add_benchmark!(params, batches, liquidity, Liquidity);
			add_benchmark!(params, batches, thea_executor, TheaExecutor); //TheaExecutor: thea_executor
			add_benchmark!(params, batches, thea, Thea);
			add_benchmark!(params, batches, thea_message_handler, TheaMH);
			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

#[cfg(test)]
mod tests {
	use frame_system::offchain::CreateSignedTransaction;

	use super::*;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}
}
