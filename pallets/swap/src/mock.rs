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

use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, SortedMembers},
	PalletId, RuntimeDebug,
};
use frame_system::{self as system, EnsureSignedBy};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};

use crate::{
	mock::sp_api_hidden_includes_construct_runtime::hidden_include::traits::{
		OnFinalize, OnInitialize,
	},
	pallet as pallet_amm, CurrencyId,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

#[derive(
	Encode,
	Decode,
	Default,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	MaxEncodedLen,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Hash))]
pub struct AccountId(pub u64);

impl sp_std::fmt::Display for AccountId {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<u64> for AccountId {
	fn from(account_id: u64) -> Self {
		Self(account_id)
	}
}

pub const ALICE: AccountId = AccountId(1);
pub const BOB: AccountId = AccountId(2);
pub const CHARLIE: AccountId = AccountId(3);
pub const EVE: AccountId = AccountId(4);
pub const FRANK: AccountId = AccountId(5);
pub const PROTOCOL_FEE_RECEIVER: AccountId = AccountId(99);
pub const DOT: CurrencyId = 10;
pub const SDOT: CurrencyId = 11;
pub const KSM: CurrencyId = 12;
pub const GLMR: CurrencyId = 13;
pub const PARA: CurrencyId = 14;
pub const SAMPLE_LP_TOKEN: CurrencyId = 42;
pub const SAMPLE_LP_TOKEN_2: CurrencyId = 43;

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
		AssetHandler: asset_handler::pallet::{Pallet, Call, Storage, Event<T>},
		ChainBridge: chainbridge::{Pallet, Storage, Call, Event<T>},
		Swap: pallet_amm::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ();
	type AssetId = u128;
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ChainId: u8 = 1;
	pub const ParachainNetworkId: u8 = 1;
	pub const ProposalLifetime: u64 = 1000;
	pub const ChainbridgePalletId: PalletId = PalletId(*b"CSBRIDGE");
}

impl chainbridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type BridgeCommitteeOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type Proposal = RuntimeCall;
	type BridgeChainId = ChainId;
	type ProposalLifetime = ProposalLifetime;
}

impl asset_handler::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type AssetManager = Assets;
	type AssetCreateUpdateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type NativeCurrencyId = ();
	type TreasuryPalletId = ChainbridgePalletId;
	type ParachainNetworkId = ParachainNetworkId;
	type PDEXHolderAccount = PDEXHolderAccount;
	type WeightInfo = asset_handler::weights::WeightInfo<Test>;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
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

pub const PDEX: Balance = 1_000_000_000_000;

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * PDEX;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const LockPeriod: u64 = 201600;
	pub const MaxRelayers: u32 = 3;
}

parameter_types! {
	pub const PolkadexAssetId: u128 = 1000;
	pub const PDEXHolderAccount: AccountId = ALICE;
}

//Install Swap pallet
parameter_types! {
	pub const SwapPalletId: PalletId = PalletId(*b"sw/accnt");
	pub DefaultLpFee: Permill = Permill::from_rational(30u32, 10000u32);
	pub OneAccount: AccountId = ALICE;
	pub DefaultProtocolFee: Permill = Permill::from_rational(0u32, 10000u32);
	pub const MinimumLiquidity: u128 = 1_000u128;
	pub const MaxLengthRoute: u8 = 10;
}

pub struct AliceCreatePoolOrigin;

impl SortedMembers<AccountId> for AliceCreatePoolOrigin {
	fn sorted_members() -> Vec<AccountId> {
		vec![ALICE]
	}
}

impl pallet_amm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Assets = AssetHandler;
	type PalletId = SwapPalletId;
	type LockAccountId = OneAccount;
	type CreatePoolOrigin = EnsureSignedBy<AliceCreatePoolOrigin, AccountId>;
	type ProtocolFeeUpdateOrigin = EnsureSignedBy<AliceCreatePoolOrigin, AccountId>;
	type LpFee = DefaultLpFee;
	type MinimumLiquidity = MinimumLiquidity;
	type MaxLengthRoute = MaxLengthRoute;
	type GetNativeCurrencyId = PolkadexAssetId;
	type WeightInfo = super::weights::WeightInfo<Test>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, 100_000_000_000_000),
			(BOB, 100_000_000_000_000),
			(CHARLIE, 1000_000_000_000_000),
			(EVE, 1000_000_000_000_000),
			(FRANK, 1000_000_000_000_000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		Assets::force_create(RuntimeOrigin::root(), DOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), SDOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), KSM.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), GLMR.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), PARA.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), SAMPLE_LP_TOKEN.into(), ALICE, true, 1)
			.unwrap();
		Assets::force_create(RuntimeOrigin::root(), SAMPLE_LP_TOKEN_2.into(), ALICE, true, 1)
			.unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), ALICE, 100_000_000).unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), BOB, 100_000_000_000_000_000_000)
			.unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), CHARLIE, 1000_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), EVE, 1000_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), FRANK, 100_000_000_000_000_000_000)
			.unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), SDOT.into(), ALICE, 100_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), SDOT.into(), BOB, 100_000_000_000_000_000_000)
			.unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), SDOT.into(), CHARLIE, 1000_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), SDOT.into(), EVE, 1000_000_000).unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), ALICE, 100_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), BOB, 100_000_000).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), FRANK, 100_000_000_000_000_000_000)
			.unwrap();
	});

	ext
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		AssetHandler::on_initialize(System::block_number());
	}
}
