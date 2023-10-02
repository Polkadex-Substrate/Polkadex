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

use crate as xcm_helper;
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use frame_system as system;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use thea::ecdsa::{AuthorityId, AuthoritySignature};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		XcmHelper: xcm_helper,
		TheaMessageHandler: thea_message_handler,
		XToken: orml_xtokens
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = u32;
	type Block = Block;
}

use frame_support::{traits::AsEnsureOriginWithArg, PalletId};
use frame_system::{EnsureRoot, EnsureSigned};

pub const TOKEN: u128 = 1_000_000_000_000;

parameter_types! {
	pub const ExistentialDeposit: u128 = TOKEN;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
}

parameter_types! {
	pub const TheaMaxAuthorities: u32 = 10;
}

impl thea_message_handler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type TheaId = AuthorityId;
	type Signature = AuthoritySignature;
	type MaxAuthorities = TheaMaxAuthorities;
	type Executor = XcmHelper;
	type WeightInfo = thea_message_handler::weights::WeightInfo<Test>;
}

parameter_types! {
	pub const AssetHandlerPalletId: PalletId = PalletId(*b"XcmHandl");
	pub const WithdrawalExecutionBlockDiff: u32 = 1000;
	pub const NativeAssetId: u128 = 100;
}

impl xcm_helper::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AccountIdConvert = ();
	type Assets = Assets;
	type AssetId = u128;
	type Currency = Balances;
	type AssetCreateUpdateOrigin = EnsureRoot<Self::AccountId>;
	type Executor = TheaMessageHandler;
	type AssetHandlerPalletId = AssetHandlerPalletId;
	type WithdrawalExecutionBlockDiff = WithdrawalExecutionBlockDiff;
	type ParachainId = ();
	type SubstrateNetworkId = ();
	type NativeAssetId = NativeAssetId;
	type WeightInfo = crate::weights::WeightInfo<Test>;
}

parameter_types! {
	pub const AssetDeposit: u128 = 100;
	pub const ApprovalDeposit: u128 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u128 = 10;
	pub const MetadataDepositPerByte: u128 = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u128;
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureSigned<Self::AccountId>;
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

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(1u128)
	};
}

use cumulus_primitives_core::{GlobalConsensus, InteriorMultiLocation, Parachain};
use xcm::latest::MultiLocation;
use xcm_builder::{
	test_utils::{NetworkId, X2},
	FixedWeightBounds,
};

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: u64 = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
	pub Ancestry: xcm::latest::MultiLocation = MultiLocation::default();
	pub MaxAssetsForTransfer: usize = 2;
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(2040));
}

impl orml_xtokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type CurrencyId = u128;
	type CurrencyIdConvert = ();
	type AccountIdToMultiLocation = ();
	type SelfLocation = ();
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = ();
	type MultiLocationsFilter = ();
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = ();
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
	type UniversalLocation = UniversalLocation;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
