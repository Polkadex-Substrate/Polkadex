use crate::{
	mock,
	mock::{new_test_ext, Test, *},
	pallet::*,
	AssetIdOf,
	ChargeAssetTransactionPayment
};
use frame_support::{assert_noop, assert_ok, weights::{DispatchInfo,Weight} };
use polkadex_primitives::{AccountId, AssetId, UNIT_BALANCE};
use sp_runtime::DispatchError::BadOrigin;

pub const ALICE_ACCOUNT_RAW_ID: [u8; 32] = [0; 32];

fn get_alice_account() -> AccountId {
	AccountId::new(ALICE_ACCOUNT_RAW_ID)
}

#[test]
fn allow_list_token_for_fees() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset));
		let mut vec = Vec::new();
		vec.insert(0, asset);
		assert_eq!(<AllowedAssets<Test>>::get(), vec);
	});
}

#[test]
fn allow_list_token_for_fees_with_bad_origin() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		assert_noop!(
			AssetsTransactionPayment::allow_list_token_for_fees(Origin::none(), asset),
			BadOrigin
		);
	});
}

#[test]
fn block_token_for_fees_when_token_not_allowlisted() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		//ToDo: Remove comment after fix
		// assert_noop!(AssetsTransactionPayment::block_token_for_fees(Origin::root(), Error::<Test>::TokenNotAllowlisted));
	});
}


#[test]
fn block_token_for_fees() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset));
		assert_ok!(AssetsTransactionPayment::block_token_for_fees(Origin::root(), asset));
		let mut vec: Vec<u128> = Vec::new();
		assert_eq!(<AllowedAssets<Test>>::get(), vec);
	});
}


pub fn info_from_weight(w: Weight) -> DispatchInfo {
	// pays_fee: Pays::Yes -- class: DispatchClass::Normal
	DispatchInfo { weight: w, ..Default::default() }
}
// const CALL: &<Test as frame_system::Config>::Call =
// 	&Call::Balances(BalancesCall::transfer { dest: 2, value: 69 });


#[test]
fn withdraw_fee() {
	new_test_ext().execute_with(|| {
		let asset_id = 1_u128;
		let tip = 0;
		let signature_scheme = 0;
		let weight = 5;
		let account = get_alice_account();
		let len = 0_usize;
		let charge_asset_transaction = ChargeAssetTransactionPayment::<Test> {
			asset_id,
			tip,
			signature_scheme
		};

		charge_asset_transaction.withdraw_fee(&account, AssetsTransactionPayment::Call, &info_from_weight(weight), len);


	});
}
