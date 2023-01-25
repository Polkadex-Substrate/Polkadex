use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use polkadex_primitives::{AccountId, AssetId};
use sp_runtime::{DispatchError::BadOrigin, SaturatedConversion};
pub const ALICE_ACCOUNT_RAW_ID: [u8; 32] = [0; 32];

fn get_alice_account() -> AccountId {
	AccountId::new(ALICE_ACCOUNT_RAW_ID)
}

#[test]
fn register_pallet_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(Origin::root()));
		assert_eq!(<PalletRegister<Test>>::get(), true);
	});
}

#[test]
fn try_to_register_pallet_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(Origin::root()));
		assert_noop!(
			Liquidity::register_account(Origin::root()),
			Error::<Test>::PalletAlreadyRegistered
		);
	});
}

#[test]
fn register_account_with_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(Liquidity::register_account(Origin::none()), BadOrigin,);
		assert_noop!(Liquidity::register_account(Origin::signed(get_alice_account())), BadOrigin,);
	});
}

#[test]
fn deposit() {
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(Origin::root()));
		assert_ok!(Liquidity::deposit_to_orderbook(
			Origin::root(),
			AssetId::polkadex,
			100_u128.saturated_into()
		));
	});
}

#[test]
fn deposit_with_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				Origin::none(),
				AssetId::polkadex,
				100_u128.saturated_into()
			),
			BadOrigin
		);
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				Origin::signed(get_alice_account()),
				AssetId::polkadex,
				100_u128.saturated_into()
			),
			BadOrigin
		);
	});
}

#[test]
fn deposit_when_pallet_not_register() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				Origin::root(),
				AssetId::polkadex,
				100_u128.saturated_into()
			),
			Error::<Test>::PalletAccountNotRegistered
		);
	});
}

#[test]
fn withdraw() {
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(Origin::root()));
		assert_ok!(Liquidity::withdraw_from_orderbook(
			Origin::root(),
			AssetId::polkadex,
			100_u128.saturated_into(),
			true
		));
	});
}

#[test]
fn withdraw_with_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				Origin::none(),
				AssetId::polkadex,
				100_u128.saturated_into(),
				true
			),
			BadOrigin
		);
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				Origin::signed(get_alice_account()),
				AssetId::polkadex,
				100_u128.saturated_into(),
				true
			),
			BadOrigin
		);
	});
}

#[test]
fn withdraw_when_pallet_not_register() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				Origin::root(),
				AssetId::polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::PalletAccountNotRegistered
		);
	});
}
