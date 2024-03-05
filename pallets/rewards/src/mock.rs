// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
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

use crate::{pallet as rewards, *};
use frame_support::{
    parameter_types,
    traits::{ConstU64, OnTimestampSet},
};
use frame_system as system;
use polkadex_primitives::{AccountId, Moment, Signature};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};
use sp_std::convert::{TryFrom, TryInto};

use frame_support::{traits::AsEnsureOriginWithArg, PalletId};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

// For testing the pallet, we construct a mock Runtime.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Timestamp: pallet_timestamp,
        Rewards: rewards
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
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
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
    type Nonce = u32;
    type Block = Block;
}

pub const PDEX: Balance = 1_000_000_000_000;

parameter_types! {
    pub const ExistentialDeposit: Balance = PDEX;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Test>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type MaxHolds = ();
    type MaxFreezes = ();
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

use sp_std::cell::RefCell;
thread_local! {
    pub static CAPTURED_MOMENT: RefCell<Option<Moment>> = RefCell::new(None);
}

pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
    fn on_timestamp_set(moment: Moment) {
        CAPTURED_MOMENT.with(|x| *x.borrow_mut() = Some(moment));
    }
}

parameter_types! {
    pub const RewardsPalletId: PalletId = PalletId(*b"REWARDSQ");
}

impl rewards::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = RewardsPalletId;
    type NativeCurrency = Balances;
    type Public = <Signature as sp_runtime::traits::Verify>::Signer;
    type Signature = Signature;
    type GovernanceOrigin = EnsureRoot<sp_runtime::AccountId32>;
    type WeightInfo = crate::weights::WeightInfo<Test>;
}

impl pallet_timestamp::Config for Test {
    type Moment = Moment;
    type OnTimestampSet = MockOnTimestampSet;
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
