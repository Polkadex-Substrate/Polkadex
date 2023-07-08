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

//! Tests for pallet-ocex.

use crate::*;
use frame_support::{assert_noop, assert_ok, bounded_vec};
use polkadex_primitives::{
	assets::AssetId, ingress::IngressMessages, withdrawal::Withdrawal, UNIT_BALANCE,
};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use sp_std::collections::btree_map::BTreeMap;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use frame_system::EventRecord;

use polkadex_primitives::{AccountId, AssetsLimit};
use rust_decimal::Decimal;
use sp_core::{bounded::BoundedBTreeSet, ByteArray, Pair};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use sp_runtime::{AccountId32, DispatchError::BadOrigin, SaturatedConversion, TokenError};

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

//Alice Account
pub const ALICE_MAIN_ACCOUNT_RAW_ID: [u8; 32] = [6u8; 32];
pub const ALICE_PROXY_ACCOUNT_RAW_ID: [u8; 32] = [7u8; 32];

fn get_alice_accounts() -> (AccountId32, AccountId32) {
	(AccountId::new(ALICE_MAIN_ACCOUNT_RAW_ID), AccountId::new(ALICE_PROXY_ACCOUNT_RAW_ID))
}

#[test]
fn test_ocex_submit_snapshot() {
	let authorities = vec![
		AuthorityId::from_slice(&hex::decode("a2cb69bc369821987a80f3441eb868e5d34ba02ef437e1eb9d5fb019994bff99b5a\
		43afecbdffb18412d80d12ed411901178c52ec8a7ad5d2a1cd40cd346af4dc92617b72b2577634b44700691ee322bf012e4795041c80117de530433c4966d").unwrap()).unwrap(),
		AuthorityId::from_slice(&hex::decode("b335a9cc86c14bb3befa2337b43000f8291c06c196c9003b24b25de75a6369f1c9df\
		67e65a0da38a3339779a2825c6871506c86fe0250316cceff08b119d559312533be83ba62093908744f4e6c4bd37c71c30719e1aaf0428de966e61847d23").unwrap()).unwrap(),
		AuthorityId::from_slice(&hex::decode("aee672d32bf85ef55c5fecedc0cc4c17ab828e4e30cada4a565fc0136c4adc1cdad5\
		6efb96d52ac19d84b43f289ad456127ea3d09af51cef4ff0776375c1ea0d2b6c42dd095119ff371e8d3d56f44eca23811d19298755dba7627fd61a3f0c9e").unwrap()).unwrap()
	];

	let signature: [u8; 48] = hex::decode(
		"927d\
		5700dfe641117ffa23fd553926a20b603d947200160f1f43b892cf007aa62bf4a9a8aaf067fdaa431b44ddaed596",
	)
	.unwrap()
	.try_into()
	.unwrap();
	let snapshot1 = SnapshotSummary {
		validator_set_id: 0,
		snapshot_id: 114,
		state_hash: H256::from_slice(
			&hex::decode("bc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a")
				.unwrap(),
		),
		worker_nonce: 1104,
		state_change_id: 1104,
		last_processed_blk: 1103,
		withdrawals: vec![],
		public: (),
	};

	let signature: [u8; 48] = hex::decode(
		"a92ba\
		a06af18d2b96713ef62951562c9585af5fadb041ef1ff3279f2d75910fefd74f96dfed112f0c99354eea23a184e",
	)
	.unwrap()
	.try_into()
	.unwrap();
	let snapshot2 = SnapshotSummary {
		validator_set_id: 0,
		snapshot_id: 114,
		state_hash: H256::from_slice(
			&hex::decode("bc36789e7a1e281436464229828f817d6612f7b477d66591ff96a9e064bcc98a")
				.unwrap(),
		),
		worker_nonce: 1104,
		state_change_id: 1104,
		last_processed_blk: 1103,
		withdrawals: vec![],
		public: (),
	};

	new_test_ext().execute_with(|| {
		<Authorities<Test>>::insert(0, ValidatorSet::new(authorities, 0));
		<SnapshotNonce<Test>>::put(113);
		OCEX::validate_snapshot(&snapshot1).unwrap();
		OCEX::validate_snapshot(&snapshot2).unwrap();
		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot1));
		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot2));
		assert_eq!(<SnapshotNonce<Test>>::get(), 114);
	});
}

#[test]
fn test_register_main_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), false);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		let account_info = Accounts::<Test>::get(account_id.clone()).unwrap();
		assert_eq!(account_info.proxies.len(), 1);
		assert_eq!(account_info.proxies[0], account_id.clone());
		assert_last_event::<Test>(
			crate::Event::MainAccountRegistered {
				main: account_id.clone(),
				proxy: account_id.clone(),
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::RegisterUser(account_id.clone(), account_id.clone());
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	});
}

#[test]
fn test_register_main_account_main_account_already_exists() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		assert_noop!(
			OCEX::register_main_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.clone().into()
			),
			Error::<Test>::MainAccountAlreadyRegistered
		);
	});
}

#[test]
fn test_register_main_account_bad_origin() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_main_account(RuntimeOrigin::root(), account_id.clone().into()),
			BadOrigin
		);
		assert_noop!(
			OCEX::register_main_account(RuntimeOrigin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account_main_account_not_found() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.into()
			),
			Error::<Test>::MainAccountNotFound
		);
	});
}
#[test]
fn test_add_proxy_account_exchange_state_not_operational() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_add_proxy_account_proxy_limit_exceeded() {
	let account_id = create_account_id();
	let proxy_account = create_proxy_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				proxy_account.clone().into()
			),
			Error::<Test>::ProxyLimitExceeded
		);
	})
}

#[test]
fn test_add_proxy_account_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(RuntimeOrigin::root(), account_id.clone().into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::add_proxy_account(RuntimeOrigin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_last_event::<Test>(
			crate::Event::MainAccountRegistered {
				main: account_id.clone(),
				proxy: account_id.clone(),
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::AddProxy(account_id.clone(), account_id.clone());
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[2], event);
	});
}

#[test]
fn test_register_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Polkadex,
				10001_u128.into(),
				100_u128.into(),
				10001_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::BothAssetsCannotBeSame
		);
	});
}

#[test]
fn test_register_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Polkadex,
				10001_u128.into(),
				100_u128.into(),
				10001_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_register_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::none(),
				AssetId::Polkadex,
				AssetId::Polkadex,
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			BadOrigin
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Polkadex,
				AssetId::Polkadex,
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			BadOrigin
		);
	});
}

#[test]
fn test_register_trading_pair_value_zero() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				0_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigCannotBeZero
		);
	});
}

#[test]
fn test_register_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_eq!(
			TradingPairs::<Test>::contains_key(AssetId::Asset(10), AssetId::Asset(20)),
			true
		);
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			true
		);
		assert_last_event::<Test>(
			crate::Event::TradingPairRegistered {
				base: AssetId::Asset(10),
				quote: AssetId::Asset(20),
			}
			.into(),
		);
		let trading_pair =
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	});
}

#[test]
fn test_register_trading_pair_amount_overflow() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_update_trading_pair_amount_overflow() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			10000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_register_trading_pair_trading_pair_already_registered() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairAlreadyRegistered
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(20),
				AssetId::Asset(10),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairAlreadyRegistered
		);
	});
}

#[test]
fn test_update_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_ok!(OCEX::update_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_last_event::<Test>(
			crate::Event::TradingPairUpdated {
				base: AssetId::Asset(10),
				quote: AssetId::Asset(20),
			}
			.into(),
		);
		let trading_pair =
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		let event: IngressMessages<AccountId32> = IngressMessages::UpdateTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[3], event);
	});
}

#[test]
fn test_update_trading_pair_with_less_than_min_volume() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			AssetId::Asset(1),
			10001_u128.into(),
			100_u128.into(),
			10001_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			AssetId::Asset(1),
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Asset(1),
				10000_u128.into(),
				100_u128.into(),
				10000_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigUnderflow
		);
	});
}

#[test]
fn test_update_trading_pair_trading_pair_not_registered() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairNotRegistered
		);
	});
}

#[test]
fn test_update_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_update_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::none(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			BadOrigin
		);
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			BadOrigin
		);
	});
}

#[test]
fn test_register_trading_pair_volume_too_low() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Asset(1),
				10000_u128.into(),
				100_u128.into(),
				10000_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigUnderflow
		);
	});
}

#[test]
fn test_update_trading_pair_value_zero() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				0_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigCannotBeZero
		);
	});
}

#[test]
fn test_deposit_unknown_asset() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let asset_id = AssetId::Asset(10);
		allowlist_token(asset_id);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				asset_id,
				100_u128.into()
			),
			TokenError::UnknownAsset
		);
	});
}
#[test]
fn test_deposit_exchange_not_operational() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Asset(10),
				100_u128.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_deposit_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));

		assert_noop!(
			OCEX::deposit(RuntimeOrigin::root(), AssetId::Asset(10), 100_u128.into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::deposit(RuntimeOrigin::none(), AssetId::Asset(10), 100_u128.into()),
			BadOrigin
		);
	});
}

#[test]
fn test_deposit_account_not_registered() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Asset(10));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Asset(10),
				100_u128.into()
			),
			Error::<Test>::AccountNotRegistered
		);
	});
}

#[test]
fn test_deposit() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000000000000000000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_ok!(OCEX::deposit(
			RuntimeOrigin::signed(account_id.clone().into()),
			AssetId::Polkadex,
			100_u128.into()
		));
		// Balances after deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			9999999999999999999900
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 100);
		assert_last_event::<Test>(
			crate::Event::DepositSuccessful {
				user: account_id.clone(),
				asset: AssetId::Polkadex,
				amount: 100_u128,
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::Deposit(account_id, AssetId::Polkadex, Decimal::new(10, 11));
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[2], event);
	});
}

#[test]
fn test_deposit_large_value() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Polkadex,
				1_000_000_000_000_000_000_000_000_0000
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_deposit_assets_overflow() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_ok!(OCEX::deposit(
			RuntimeOrigin::signed(account_id.clone().into()),
			AssetId::Polkadex,
			1_000_000_000_000_000_000_000_000_000
		));
		let large_value: Decimal = Decimal::MAX;
		mint_into_account_large(account_id.clone());
		// Directly setting the storage value, found it very difficult to manually fill it up
		TotalAssets::<Test>::insert(
			AssetId::Polkadex,
			large_value.saturating_sub(Decimal::from_u128(1).unwrap()),
		);

		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Polkadex,
				10_u128.pow(20)
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_open_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}
#[test]
fn test_open_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_open_trading_pair_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(20)),
			Error::<Test>::TradingPairNotFound
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_open_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::none(), AssetId::Asset(10), AssetId::Asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::open_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_open_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::open_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			true
		);
		let trading_pair = OCEX::trading_pairs(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::OpenTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	})
}

#[test]
fn test_close_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_close_trading_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_close_trading_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(20)),
			Error::<Test>::TradingPairNotFound
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_close_trading_trading_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::none(), AssetId::Asset(10), AssetId::Asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::close_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_close_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			false
		);
		let trading_pair = OCEX::trading_pairs(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::ShutdownTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::CloseTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[2], event);
	})
}

#[test]
fn test_update_trading_pair_with_closed_operational_status() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_0000_0000_u128.into(),
				1_000_000_000_000_000_u128.into(),
				1_000_000_u128.into(),
				1_000_000_000_000_000_u128.into(),
				1_000_0000_u128.into(),
				1_0000_000_u128.into(),
			),
			Error::<Test>::TradingPairIsNotClosed
		);
	})
}

#[test]
fn collect_fees_unexpected_behaviour() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		assert_ok!(OCEX::collect_fees(RuntimeOrigin::root(), 100, account_id.clone().into()));

		assert_last_event::<Test>(
			crate::Event::FeesClaims { beneficiary: account_id, snapshot_id: 100 }.into(),
		);
	});
}

#[test]
fn test_collect_fees_decimal_overflow() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		let max_fees = create_max_fees::<Test>();
		FeesCollected::<Test>::insert::<u64, BoundedVec<Fees, AssetsLimit>>(
			0,
			bounded_vec![max_fees],
		);
		assert_noop!(
			OCEX::collect_fees(RuntimeOrigin::root(), 0, account_id.into()),
			Error::<Test>::FeesNotCollectedFully
		);
	})
}

#[test]
fn collect_fees() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	let mut t = new_test_ext();
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());
		let initial_balance = 10_000_000_000 * UNIT_BALANCE;
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance
		);

		let (mut snapshot, _public) = get_dummy_snapshot(1);

		snapshot.withdrawals[0].fees = Decimal::from_f64(0.1).unwrap();

		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot.clone()));

		assert_ok!(OCEX::claim_withdraw(
			RuntimeOrigin::signed(account_id.clone().into()),
			1,
			account_id.clone()
		));

		// Balances after withdrawal
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance + UNIT_BALANCE
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance - UNIT_BALANCE
		);

		assert_ok!(OCEX::collect_fees(RuntimeOrigin::root(), 1, account_id.clone().into()));

		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance +
				UNIT_BALANCE + snapshot.withdrawals[0]
				.fees
				.saturating_mul(Decimal::from(UNIT_BALANCE))
				.to_u128()
				.unwrap()
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance -
				UNIT_BALANCE - snapshot.withdrawals[0]
				.fees
				.saturating_mul(Decimal::from(UNIT_BALANCE))
				.to_u128()
				.unwrap()
		);
	});
}

#[test]
fn test_collect_fees_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::collect_fees(
				RuntimeOrigin::signed(account_id.clone()),
				100,
				account_id.clone().into()
			),
			BadOrigin
		);

		assert_noop!(
			OCEX::collect_fees(RuntimeOrigin::signed(account_id.clone()), 100, account_id.into()),
			BadOrigin
		);
	});
}

#[test]
fn withdrawal_when_exchange_not_operational() {
	let (alice_account_id, proxy_account_id) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_account_id.clone(),
				proxy_account_id,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn withdrawal_when_token_not_allowlisted() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::TokenNotAllowlisted
		);
	});
}

#[test]
fn withdrawal_when_account_not_register() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::AccountNotRegistered
		);
	});
}

#[test]
fn withdrawal_with_overflow_amount() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);

		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(alice_main_account.clone().into()),
			alice_proxy_account.clone().into()
		));

		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				(WITHDRAWAL_MAX + 1).saturated_into(),
				true
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn withdrawal() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);

		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(alice_main_account.clone().into()),
			alice_proxy_account.clone().into()
		));

		assert_ok!(OCEX::withdrawal_from_orderbook(
			alice_main_account.clone(),
			alice_proxy_account.clone(),
			AssetId::Polkadex,
			100_u128.saturated_into(),
			true
		));
		let blk = frame_system::Pallet::<Test>::current_block_number();
		//assert ingress message
		assert_eq!(
			OCEX::ingress_messages(blk)[2],
			IngressMessages::DirectWithdrawal(
				alice_proxy_account,
				AssetId::Polkadex,
				Decimal::new(100, 12),
				true
			)
		);

		//assert event
		assert_last_event::<Test>(
			crate::Event::WithdrawFromOrderbook(alice_main_account, AssetId::Polkadex, 100_u128)
				.into(),
		);
	});
}

// P.S. This was to apply a DDOS attack and see the response in the mock environment
#[ignore]
#[test]
fn collect_fees_ddos() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		for x in 0..10000000 {
			assert_ok!(OCEX::collect_fees(
				RuntimeOrigin::signed(account_id.clone().into()),
				x,
				account_id.clone().into()
			));
		}
	});
}

#[test]
fn test_submit_snapshot_snapshot_nonce_error() {
	new_test_ext().execute_with(|| {
		let (mut snapshot, _public) = get_dummy_snapshot(0);
		snapshot.snapshot_id = 2; // Wrong nonce
		assert_noop!(
			OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot),
			Error::<Test>::SnapshotNonceError
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 0);
	});
}

fn get_dummy_snapshot(
	withdrawals_len: usize,
) -> (SnapshotSummary<AccountId,AuthorityId>, bls_primitives::Public) {
	let main = create_account_id();

	let mut withdrawals = vec![];
	for _ in 0..withdrawals_len {
		withdrawals.push(Withdrawal {
			main_account: main.clone(),
			amount: Decimal::one(),
			asset: AssetId::Polkadex,
			fees: Default::default(),
			stid: 0,
			worker_nonce: 0,
		})
	}

	let mut snapshot = SnapshotSummary {
		validator_set_id: 0,
		snapshot_id: 1,
		state_hash: Default::default(),
		worker_nonce: 1,
		state_change_id: 1,
		last_processed_blk: 1,
		withdrawals,
		public: None,
	};
	let (pair, _seed) = bls_primitives::Pair::generate();
	snapshot.aggregate_signature = Some(pair.sign(&snapshot.sign_data()));

	(snapshot, pair.public())
}

#[test]
fn test_submit_snapshot_bad_origin() {
	new_test_ext().execute_with(|| {
		let (snapshot, _public) = get_dummy_snapshot(1);
		assert_noop!(OCEX::validate_snapshot(&snapshot), InvalidTransaction::Custom(11));
	});
}

#[test]
fn test_submit_snapshot() {
	let _account_id = create_account_id();
	let mut t = new_test_ext();
	t.execute_with(|| {
		let (mut snapshot, _public) = get_dummy_snapshot(1);
		snapshot.withdrawals[0].fees = Decimal::from_f64(1.0).unwrap();
		let mut withdrawal_map = BTreeMap::new();
		for withdrawal in &snapshot.withdrawals {
			match withdrawal_map.get_mut(&withdrawal.main_account) {
				None => {
					withdrawal_map
						.insert(withdrawal.main_account.clone(), vec![withdrawal.clone()]);
				},
				Some(list) => {
					list.push(withdrawal.clone());
				},
			}
		}
		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot.clone()));

		assert_eq!(Withdrawals::<Test>::contains_key(1), true);
		assert_eq!(Withdrawals::<Test>::get(1), withdrawal_map.clone());
		assert_eq!(FeesCollected::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::get(1), snapshot.clone());
		assert_eq!(SnapshotNonce::<Test>::get(), 1);
		let onchain_events =
			vec![polkadex_primitives::ocex::OnChainEvents::OrderbookWithdrawalProcessed(
				1,
				snapshot.withdrawals.clone(),
			)];
		assert_eq!(OnChainEvents::<Test>::get(), onchain_events);
		// Checking for redundant data inside snapshot
		assert_eq!(Snapshots::<Test>::get(1).withdrawals, snapshot.withdrawals);
	})
}

#[test]
fn test_withdrawal_invalid_withdrawal_index() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::claim_withdraw(
				RuntimeOrigin::signed(account_id.clone().into()),
				1,
				account_id.clone()
			),
			Error::<Test>::InvalidWithdrawalIndex
		);
	});
}

#[test]
fn test_withdrawal() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	let mut t = new_test_ext();
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());

		let initial_balance = 10_000_000_000 * UNIT_BALANCE;
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance
		);

		let (snapshot, _public) = get_dummy_snapshot(1);

		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot.clone()));

		assert_ok!(OCEX::claim_withdraw(
			RuntimeOrigin::signed(account_id.clone().into()),
			1,
			account_id.clone()
		));
		// Balances after withdrawal
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance + UNIT_BALANCE // Increased by 1
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance - UNIT_BALANCE, // Dec
		);
		let withdrawal_claimed: polkadex_primitives::ocex::OnChainEvents<AccountId> =
			polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
				1,
				account_id.clone().into(),
				bounded_vec![snapshot.withdrawals[0].clone()],
			);
		assert_eq!(OnChainEvents::<Test>::get()[1], withdrawal_claimed);
	});
}
use orderbook_primitives::{Fees, H128};
use sp_runtime::traits::{BlockNumberProvider, One};

#[test]
fn test_withdrawal_bad_origin() {
	new_test_ext().execute_with(|| {
		let account_id = create_account_id();
		assert_noop!(OCEX::claim_withdraw(RuntimeOrigin::root(), 1, account_id.clone()), BadOrigin);

		assert_noop!(OCEX::claim_withdraw(RuntimeOrigin::none(), 1, account_id.clone()), BadOrigin);
	});
}

#[test]
pub fn test_allowlist_and_blacklist_token() {
	new_test_ext().execute_with(|| {
		let _account_id = create_account_id();
		let new_token = AssetId::Asset(1);
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(allowlisted_tokens.contains(&new_token));
		assert_ok!(OCEX::remove_allowlisted_token(RuntimeOrigin::root(), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(!allowlisted_tokens.contains(&new_token));
	});
}

#[test]
pub fn test_allowlist_with_limit_reaching_returns_error() {
	new_test_ext().execute_with(|| {
		let _account_id = create_account_id();
		let mut allowlisted_assets: BoundedBTreeSet<AssetId, AllowlistedTokenLimit> =
			BoundedBTreeSet::new();
		for ele in 0..50 {
			assert_ok!(allowlisted_assets.try_insert(AssetId::Asset(ele)));
		}
		assert_eq!(allowlisted_assets.len(), 50);
		<AllowlistedToken<Test>>::put(allowlisted_assets);
		let new_token = AssetId::Asset(100);
		assert_noop!(
			OCEX::allowlist_token(RuntimeOrigin::root(), new_token),
			Error::<Test>::AllowlistedTokenLimitReached
		);
	});
}

use polkadex_primitives::ingress::{HandleBalance, HandleBalanceLimit};
use crate::sr25519::AuthorityId;

#[test]
fn test_set_balances_with_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		let bounded_vec_for_alice: BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit> =
			BoundedVec::try_from(vec_of_balances).unwrap();

		assert_noop!(OCEX::set_balances(RuntimeOrigin::none(), bounded_vec_for_alice), BadOrigin);
	});
}

#[test]
pub fn test_set_balances_when_exchange_is_not_pause() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		let bounded_vec_for_alice: BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit> =
			BoundedVec::try_from(vec_of_balances).unwrap();

		assert_noop!(
			OCEX::set_balances(RuntimeOrigin::root(), bounded_vec_for_alice),
			Error::<Test>::ExchangeOperational
		);
	});
}

#[test]
pub fn test_set_balances_when_exchange_is_pause() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), false));
		let mut vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		vec_of_balances.push(HandleBalance {
			main_account: account_id,
			asset_id: AssetId::Polkadex,
			free: 100,
			reserve: 50,
		});
		let bounded_vec_for_alice: BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit> =
			BoundedVec::try_from(vec_of_balances).unwrap();

		assert_eq!(
			OCEX::set_balances(RuntimeOrigin::root(), bounded_vec_for_alice.clone()),
			Ok(())
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(
			OCEX::ingress_messages(blk)[1],
			IngressMessages::SetFreeReserveBalanceForAccounts(bounded_vec_for_alice,)
		);
	});
}

#[test]
pub fn test_set_balances_when_bounded_vec_limits_out_of_bound() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), false));
		let mut vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		for _i in 0..1001 {
			vec_of_balances.push(HandleBalance {
				main_account: account_id.clone(),
				asset_id: AssetId::Polkadex,
				free: 100,
				reserve: 50,
			});
		}
		let bounded_vec_for_alice: Result<
			BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit>,
			Vec<HandleBalance<AccountId32>>,
		> = BoundedVec::try_from(vec_of_balances);
		assert!(bounded_vec_for_alice.is_err());
	});
}

#[test]
pub fn test_set_balances_when_bounded_vec_limits_in_bound() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), false));
		let mut vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		for _i in 0..1000 {
			vec_of_balances.push(HandleBalance {
				main_account: account_id.clone(),
				asset_id: AssetId::Polkadex,
				free: 100,
				reserve: 50,
			});
		}
		let bounded_vec_for_alice: BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit> =
			BoundedVec::try_from(vec_of_balances).unwrap();
		assert_eq!(
			OCEX::set_balances(RuntimeOrigin::root(), bounded_vec_for_alice.clone()),
			Ok(())
		);
	});
}

#[test]
fn test_remove_proxy_account_faulty_cases() {
	let (main, proxy) = get_alice_accounts();
	new_test_ext().execute_with(|| {
		// bad origin
		assert_noop!(OCEX::remove_proxy_account(RuntimeOrigin::root(), proxy.clone()), BadOrigin);
		assert_noop!(OCEX::remove_proxy_account(RuntimeOrigin::none(), proxy.clone()), BadOrigin);
		// exchange not operational
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::ExchangeNotOperational
		);
		// no main account registered
		<ExchangeState<Test>>::set(true);
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::MainAccountNotFound
		);
		// minimum one proxy required
		OCEX::register_main_account(RuntimeOrigin::signed(main.clone()), proxy.clone()).unwrap();
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::MinimumOneProxyRequired
		);
		// no proxy account found
		<Accounts<Test>>::mutate(&main, |account_info| {
			if let Some(a) = account_info {
				a.proxies.pop();
				a.proxies.try_push(main.clone()).unwrap();
				a.proxies.try_push(main.clone()).unwrap();
			} else {
				panic!("failed to mutate Accounts")
			}
		});
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main), proxy,),
			Error::<Test>::ProxyNotFound
		);
	})
}

#[test]
fn test_remove_proxy_account_proper_case() {
	let (main, proxy) = get_alice_accounts();
	new_test_ext().execute_with(|| {
		<ExchangeState<Test>>::set(true);
		OCEX::register_main_account(RuntimeOrigin::signed(main.clone()), proxy.clone()).unwrap();
		<Accounts<Test>>::mutate(&main, |account_info| {
			if let Some(a) = account_info {
				a.proxies.try_push(main.clone()).unwrap();
				a.proxies.try_push(main.clone()).unwrap();
			} else {
				panic!("failed to mutate Accounts")
			}
		});
		assert_ok!(OCEX::remove_proxy_account(RuntimeOrigin::signed(main), proxy));
	})
}

#[test]
fn test_set_snapshot_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::signed(a), 1), BadOrigin);
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::signed(b), 1), BadOrigin);
		// proper cases
		assert_ok!(OCEX::set_snapshot(RuntimeOrigin::root(), 1));
	})
}

#[test]
fn test_change_pending_withdrawal_limit_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::signed(a), 1), BadOrigin);
		assert_noop!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::signed(b), 1), BadOrigin);
		// proper cases
		assert_ok!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::root(), 1));
		// half max
		assert_ok!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::root(), u32::MAX.into()));
		// max
		assert_ok!(OCEX::change_pending_withdrawal_limit(RuntimeOrigin::root(), u64::MAX));
	})
}

#[test]
fn test_change_snapshot_interval_block_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::change_snapshot_interval_block(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(OCEX::change_snapshot_interval_block(RuntimeOrigin::signed(a), 1), BadOrigin);
		assert_noop!(OCEX::change_snapshot_interval_block(RuntimeOrigin::signed(b), 1), BadOrigin);
		// proper cases
		assert_ok!(OCEX::change_snapshot_interval_block(RuntimeOrigin::root(), 1));
		// half max
		assert_ok!(OCEX::change_snapshot_interval_block(RuntimeOrigin::root(), u32::MAX.into()));
		// max
		assert_ok!(OCEX::change_snapshot_interval_block(RuntimeOrigin::root(), u64::MAX));
	})
}

#[test]
fn test_set_exchange_state_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::none(), true), BadOrigin);
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::signed(a), true), BadOrigin);
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::signed(b), true), BadOrigin);
		// proper case
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let current = frame_system::Pallet::<Test>::current_block_number();
		assert!(<crate::IngressMessages<Test>>::get(current).len() == 1);
	})
}

#[test]
fn test_whitelist_orderbook_operator_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		let key = sp_core::ecdsa::Pair::generate().0.public();
		// bad origins
		assert_noop!(OCEX::whitelist_orderbook_operator(RuntimeOrigin::none(), key), BadOrigin);
		assert_noop!(
			OCEX::whitelist_orderbook_operator(RuntimeOrigin::signed(a.clone()), key),
			BadOrigin
		);
		assert_noop!(
			OCEX::whitelist_orderbook_operator(RuntimeOrigin::signed(b.clone()), key),
			BadOrigin
		);
		// proper case
		assert_ok!(OCEX::whitelist_orderbook_operator(RuntimeOrigin::root(), key));
		assert_eq!(<OrderbookOperatorPublicKey<Test>>::get().unwrap(), key);
	})
}

fn allowlist_token(token: AssetId) {
	let mut allowlisted_token = <AllowlistedToken<Test>>::get();
	allowlisted_token.try_insert(token).unwrap();
	<AllowlistedToken<Test>>::put(allowlisted_token);
}

fn mint_into_account(account_id: AccountId32) {
	let _result = Balances::deposit_creating(&account_id, 10000000000000000000000);
}

fn mint_into_account_large(account_id: AccountId32) {
	let _result =
		Balances::deposit_creating(&account_id, 1_000_000_000_000_000_000_000_000_000_000);
}

#[allow(dead_code)]
fn create_asset_and_credit(asset_id: u128, account_id: AccountId32) {
	assert_ok!(Assets::create(
		RuntimeOrigin::signed(account_id.clone().into()),
		asset_id.into(),
		account_id.clone().into(),
		100_u128
	));
}

fn create_account_id() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = MemoryKeystore::new();
	let account_id: AccountId32 = <(dyn Keystore + 'static)>::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}

fn create_proxy_account() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = MemoryKeystore::new();
	let account_id: AccountId32 = <(dyn Keystore + 'static)>::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter2", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}

fn create_max_fees<T: Config>() -> Fees {
	let fees: Fees = Fees { asset: AssetId::Polkadex, amount: Decimal::MAX };
	return fees
}
