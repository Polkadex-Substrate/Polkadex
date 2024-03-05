// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Tests for pallet-ocex

use crate::*;
use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU64, OnTimestampSet},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use polkadex_primitives::{Moment, Signature};
use sp_application_crypto::sp_core::H256;
use sp_core::offchain::testing::TestOffchainExt;
use sp_core::offchain::{OffchainDbExt, OffchainWorkerExt};
use sp_core::Pair;
use sp_keystore::testing::MemoryKeystore;
use sp_keystore::{Keystore, KeystoreExt};
use sp_std::cell::RefCell;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
// Reexport crate as its pallet name for construct_runtime.

type Block = frame_system::mocking::MockBlock<Test>;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		Timestamp: pallet_timestamp,
		LiqudityMining: pallet_lmp::pallet,
		OCEX: crate,
	}
);

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 64));
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_runtime::AccountId32;
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
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = u64;
	type Block = Block;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = u128;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type MaxHolds = ();
	type MaxFreezes = ();
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

parameter_types! {
	pub const ProxyLimit: u32 = 2;
	pub const OcexPalletId: PalletId = PalletId(*b"OCEX_LMP");
	pub const TreasuryPalletId: PalletId = PalletId(*b"OCEX_TRS");
	//pub const TreasuryPalletId: PalletId = PalletId(*b"OCEX_CRW");
	pub const MsPerDay: u64 = 86_400_000;
}

impl pallet_lmp::pallet::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = OcexPalletId;
	type NativeCurrency = Balances;
	type OtherAssets = Assets;
	type OCEX = OCEX;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = OcexPalletId;
	type TreasuryPalletId = TreasuryPalletId;
	type LMPRewardsPalletId = OcexPalletId;
	type NativeCurrency = Balances;
	type OtherAssets = Assets;
	type EnclaveOrigin = EnsureRoot<sp_runtime::AccountId32>;
	type AuthorityId = crate::sr25519::AuthorityId;
	type GovernanceOrigin = EnsureRoot<sp_runtime::AccountId32>;
	type CrowdSourceLiqudityMining = LiqudityMining;
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
	type RemoveItemsLimit = ();
	type AssetId = u128;
	type AssetIdParameter = parity_scale_codec::Compact<u128>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<sp_runtime::AccountId32>>;
	type ForceOrigin = EnsureRoot<sp_runtime::AccountId32>;
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

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let (pair, _seed) = sp_core::sr25519::Pair::from_phrase(
		"spider sell nice animal border success square soda stem charge caution echo",
		None,
	)
	.unwrap();
	let keystore = MemoryKeystore::new();
	keystore
		.insert(
			OCEX,
			"0xb6186f80dce7190294665ab53860de2841383bb202c562bb8b81a624351fa318",
			pair.public().as_ref(),
		)
		.unwrap();
	let validator_set_id = 0;
	let validator_set = ValidatorSet::new(vec![pair.public().into()], validator_set_id);
	ext.register_extension(KeystoreExt::new(keystore));
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.execute_with(|| {
		<Authorities<Test>>::insert(validator_set_id, validator_set);
		System::set_block_number(1)
	});
	ext
}

use sp_runtime::{
	testing::TestXt,
	traits::{Extrinsic as ExtrinsicT, IdentifyAccount, Verify},
};

type Extrinsic = TestXt<RuntimeCall, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}
