use crate::mock::*;
use frame_support::{assert_ok, assert_noop};
use crate::{Balances, TotalIssuance};
use polkadex_primitives::assets::AssetId;

use super::*;

fn set_balance<T: Config>(asset_id: AssetId, account_id: u64, amount: Balance)
{
    let value = amount;
    Balances::<Test>::insert(asset_id, account_id, &value);
    TotalIssuance::<Test>::insert(asset_id, value);
}

#[test]
fn transfer_free_balance() {
    new_tester().execute_with(|| {

        let asset_id = AssetId::POLKADEX;
        let alice: u64 = 1;
        let bob: u64 = 4;

        set_balance::<Test>(asset_id, alice.clone(), 500);
        assert_ok!(AssetsModule::transfer(Origin::signed(alice.clone()), asset_id, bob.clone(), 100));

        assert_eq!(Balances::<Test>::get(&asset_id, &alice), 400);
        assert_eq!(Balances::<Test>::get(&asset_id, &bob), 100);
        assert_eq!(TotalIssuance::<Test>::get(&asset_id), 500);
    });
}

#[test]
fn transfer_should_raise_insufficient_balance() {
    new_tester().execute_with(|| {

        let asset_id = AssetId::POLKADEX;
        let alice: u64 = 1;
        let bob: u64 = 4;

        assert_noop!(
			AssetsModule::transfer(Origin::signed(alice.clone()), asset_id, bob.clone(), 100),
			Error::<Test>::InsufficientBalance,
		);
    });
}