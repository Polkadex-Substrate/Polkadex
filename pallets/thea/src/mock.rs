// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use crate::pallet as thea;
use core::marker::PhantomData;
use frame_support::{parameter_types, PalletId};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	curve::PiecewiseLinear,
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use std::collections::{BTreeMap, BTreeSet};
use system::EnsureRoot;
use thea_primitives::thea_types::OnSessionChange;
use thea_staking::SessionChanged;


type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		ChainBridge: chainbridge::{Pallet, Storage, Call, Event<T>},
		AssetHandler: asset_handler::pallet::{Pallet, Storage, Call, Event<T>},
		Thea: thea::{Pallet, Storage, Call, Event<T>},
		TheaStaking: thea_staking::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

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
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

parameter_types! {
	pub const LockPeriod: u64 = 201600;
	pub const MaxRelayers: u32 = 3;
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

parameter_types! {
	pub const ChainId: u8 = 1;
	pub const ParachainNetworkId: u8 = 1;
	pub const ProposalLifetime: u64 = 1000;
	pub const ChainbridgePalletId: PalletId = PalletId(*b"CSBRIDGE");
}

impl chainbridge::Config for Test {
	type Event = Event;
	type BridgeCommitteeOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type Proposal = Call;
	type BridgeChainId = ChainId;
	type ProposalLifetime = ProposalLifetime;
	//type PalletId = ChainbridgePalletId;
}

parameter_types! {
	pub const PolkadexAssetId: u128 = 1000;
	pub const PDEXHolderAccount: u64 = 10u64;
}

impl asset_handler::pallet::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AssetManager = Assets;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type TreasuryPalletId = ChainbridgePalletId;
	type WeightInfo = asset_handler::weights::WeightInfo<Test>;
	type ParachainNetworkId = ParachainNetworkId;
	type PolkadexAssetId = PolkadexAssetId;
	type PDEXHolderAccount = PDEXHolderAccount;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 1;
	pub const MaxReserves: u32 = 50;
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

pub struct MockPallet(PhantomData<u32>);

impl SessionChanged for MockPallet {
	type Network = u8;
	type OnSessionChange = OnSessionChange<u64>;
	fn on_new_session(_map: BTreeMap<Self::Network, Self::OnSessionChange>) {
		// Do nothing lol
	}
	fn set_new_networks(_networks: BTreeSet<Self::Network>) {
		// Do nothing lol
	}
}

parameter_types! {
	pub const SessionLength: u64 = 7000;
	pub const UnbondingDelay: u32 = 10;
	pub const MaxUnlockChunks: u32 = 10;
	pub const CandidateBond: Balance = 1000_000_000_000;
	pub const StakingReserveIdentifier: [u8; 8] = [1u8;8];
	pub const StakingDataPruneDelay: u32 = 6;
	pub const ModerateSK: u8 = 5; // 5% of stake to slash
	pub const SevereSK: u8 = 20; // 20% of stake to slash
	pub const ReporterRewardKF: u8 = 1; // 1% of total slashed goes to each reporter
	pub const SlashingTh: u8 = 60; // 60% of threshold for slashing
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

impl thea_staking::Config for Test {
	type Event = Event;
	type SessionLength = SessionLength;
	type UnbondingDelay = UnbondingDelay;
	type MaxUnlockChunks = MaxUnlockChunks;
	type CandidateBond = CandidateBond;
	type StakingReserveIdentifier = StakingReserveIdentifier;
	type StakingDataPruneDelay = StakingDataPruneDelay;
	type SessionChangeNotifier = MockPallet;
	type ModerateSlashingCoeficient = ModerateSK;
	type SevereSlashingCoeficient = SevereSK;
	type ReportersRewardCoeficient = ReporterRewardKF;
	type SlashingThreshold = SlashingTh;
	type TreasuryPalletId = TreasuryPalletId;
	type GovernanceOrigin = EnsureRoot<u64>;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type Currency = Balances;
	type WeightInfo = thea_staking::weight::StakeWeightInfo<Test>;
}

parameter_types! {
	pub const TheaPalletId: PalletId = PalletId(*b"THBRIDGE");
	pub const WithdrawalSize: u32 = 10;
	pub const ParaId: u32 = 2040;
}

impl thea::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type TheaPalletId = TheaPalletId;
	type WithdrawalSize = WithdrawalSize;
	type ParaId = ParaId;
	type ExtrinsicSubmittedNotifier = TheaStaking;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}
