use crate::mock::*;
use frame_support::{assert_ok, assert_noop};
use sp_keyring::AccountKeyring as Keyring;
use crate::{Balances, TotalIssuance};
use polkadex_primitives::assets::AssetId;

use super::*;

fn set_balance<T>(asset_id: AssetId, account_id: &AccountId, amount: T)
    where T : Into<U256> + Copy
{
    let value = amount.into();
    Balances::<Test>::insert(asset_id, &account_id, &value);
    TotalIssuance::insert(asset_id, value);
}

#[test]
fn transfer_free_balance() {
    new_tester().execute_with(|| {

        let asset_id = AssetId::POLKADEX;
        let alice: AccountId = Keyring::Alice.into();
        let bob: AccountId = Keyring::Bob.into();

        set_balance(asset_id, &alice.clone(), 500);
        assert_ok!(TemplateModule::transfer(Origin::signed(alice.clone()), asset_id, bob.clone(), 100.into()));

        assert_eq!(Balances::<Test>::get(&asset_id, &alice), 400.into());
        assert_eq!(Balances::<Test>::get(&asset_id, &bob), 100.into());
        assert_eq!(TotalIssuance::get(&asset_id), 500.into());
    });
}

#[test]
fn transfer_should_raise_insufficient_balance() {
    new_tester().execute_with(|| {

        let asset_id = AssetId::POLKADEX;
        let alice: AccountId = Keyring::Alice.into();
        let bob: AccountId = Keyring::Bob.into();

        assert_noop!(
			TemplateModule::transfer(Origin::signed(alice.clone()), asset_id, bob.clone(), 100.into()),
			Error::<Test>::InsufficientBalance,
		);
    });
}