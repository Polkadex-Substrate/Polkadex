use crate::{
	mock::{new_test_ext, Test, *},
	pallet::AllowedAssets,
	ChargeAssetTransactionPayment,
};
use frame_support::{
	assert_noop, assert_ok,
	pallet_prelude::Pays,
	weights::{DispatchInfo, PostDispatchInfo, Weight},
};
use frame_system::Config;
use pallet_balances::Call as BalancesCall;
use polkadex_primitives::{AccountId, UNIT_BALANCE};
use sp_runtime::{traits::SignedExtension, DispatchError::BadOrigin, SaturatedConversion};

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
fn block_token_for_fees_when_token_allowlisted() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset));
		assert_eq!(<AllowedAssets<Test>>::get(), vec![asset]);
		assert_ok!(AssetsTransactionPayment::block_token_for_fees(Origin::root(), asset));
		assert_eq!(<AllowedAssets<Test>>::get(), vec![]);
	});
}

#[test]
fn block_token_for_fees_when_token_not_allowlisted() {
	new_test_ext().execute_with(|| {
		let asset = 1_u128;
		assert_ok!(AssetsTransactionPayment::block_token_for_fees(Origin::root(), asset));
		assert_eq!(<AllowedAssets<Test>>::get(), vec![]);
	});
}

pub fn info_from_weight(w: Weight) -> DispatchInfo {
	// pays_fee: Pays::Yes -- class: DispatchClass::Normal
	DispatchInfo { weight: w, ..Default::default() }
}

fn post_info_from_weight(w: Weight) -> PostDispatchInfo {
	PostDispatchInfo { actual_weight: Some(w), pays_fee: Default::default() }
}

fn info_from_pays(p: Pays) -> DispatchInfo {
	DispatchInfo { pays_fee: p, ..Default::default() }
}

fn post_info_from_pays(p: Pays) -> PostDispatchInfo {
	PostDispatchInfo { actual_weight: None, pays_fee: p }
}

fn default_post_info() -> PostDispatchInfo {
	PostDispatchInfo { actual_weight: None, pays_fee: Default::default() }
}

#[test]
fn transaction_payment_in_native_possible() {
	new_test_ext().execute_with(|| {
		let weight = 5;
		let len = 10;
		let asset_id = 0_u128;
		let balance = 1 * UNIT_BALANCE;

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 69.saturated_into(),
		});

		//don't give a tip to block author
		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip: 0 };

		let pre_dispatch_result = charge_asset_transaction_payment.pre_dispatch(
			&get_alice_account(),
			call,
			&info_from_weight(weight),
			len,
		);
		assert!(pre_dispatch_result.is_ok());
		assert_eq!(Balances::free_balance(get_alice_account()), balance - 10);

		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result.unwrap()),
			&info_from_weight(100),
			&post_info_from_weight(50),
			len,
			&Ok(())
		));
		assert_eq!(Balances::free_balance(get_alice_account()), balance - 10);

		//attach a tip to block author
		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip: 5 };
		let pre_dispatch_result = charge_asset_transaction_payment.pre_dispatch(
			&get_alice_account(),
			call,
			&info_from_weight(weight),
			len,
		);
		assert!(pre_dispatch_result.is_ok());
		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result.unwrap()),
			&info_from_weight(100),
			&post_info_from_weight(50),
			len,
			&Ok(())
		));
		assert_eq!(Balances::free_balance(get_alice_account()), balance - 10 - 15);
	})
}

#[test]
fn transaction_payment_in_asset_possible() {
	new_test_ext().execute_with(|| {
		let weight = 5;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 1 * UNIT_BALANCE;
		let fee = (weight + len as u64) * (min_balance as u64) / 10_u64;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 69.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip: 0 };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.unwrap();
		assert_eq!(
			Assets::balance(asset_id.into(), get_alice_account()),
			balance - (fee.saturated_into::<u128>() - 1)
		);

		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_weight(weight),
			&post_info_from_weight(weight),
			len,
			&Ok(())
		));
		assert_eq!(
			Assets::balance(asset_id.into(), get_alice_account()),
			balance - (fee.saturated_into::<u128>() - 1)
		);
	});
}

#[test]
fn transaction_payment_without_fee() {
	new_test_ext().execute_with(|| {
		let weight = 5;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 1 * UNIT_BALANCE;
		let fee = (weight + len as u64) * (min_balance as u64) / 10_u64;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 69.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip: 0 };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.unwrap();
		assert_eq!(
			Assets::balance(asset_id.into(), get_alice_account()),
			balance - (fee.saturated_into::<u128>() - 1)
		);

		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_weight(weight),
			&post_info_from_pays(Pays::No),
			len,
			&Ok(())
		));
		//initial balance back to users account
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance);
	});
}

#[test]
fn asset_transaction_payment_with_tip_and_refund() {
	new_test_ext().execute_with(|| {
		let weight = 100;
		let tip = 5_u128;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 1 * UNIT_BALANCE;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 69.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.unwrap();
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance - 3);

		let final_weight = 100;
		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_weight(weight),
			&post_info_from_weight(final_weight),
			len,
			&Ok(())
		));
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance - 3);
	});
}

#[test]
fn payment_from_account_with_only_assets() {
	new_test_ext().execute_with(|| {
		let weight = 5;
		let tip = 0;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 1 * UNIT_BALANCE;
		let fee = (weight + len as u64) * (min_balance as u64) / 10_u64;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 1.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.unwrap();
		assert_eq!(
			Assets::balance(asset_id.into(), get_alice_account()),
			balance - (fee.saturated_into::<u128>() - 1)
		);

		assert_ok!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_weight(weight),
			&default_post_info(),
			len,
			&Ok(())
		));
		assert_eq!(
			Assets::balance(asset_id.into(), get_alice_account()),
			balance - (fee.saturated_into::<u128>() - 1)
		);
	});
}

#[test]
fn payment_only_with_existing_sufficient_asset() {
	new_test_ext().execute_with(|| {
		let weight = 5;
		let tip = 0;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 10000 * UNIT_BALANCE;

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 1.saturated_into(),
		});

		//non existent asset
		assert!(charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.is_err());
		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };
		// pre_dispatch fails for non-sufficient asset
		assert!(charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_weight(weight), len)
			.is_err());
	});
}

#[test]
fn converted_fee_is_never_zero_if_input_fee_is_not() {
	new_test_ext().execute_with(|| {
		let tip = 0;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 1 * UNIT_BALANCE;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));
		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 1.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_pays(Pays::No), len)
			.unwrap();
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance);

		assert!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_pays(Pays::No),
			&post_info_from_pays(Pays::No),
			len,
			&Ok(())
		)
		.is_err());
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance);
	});
}

#[test]
fn post_dispatch_fee_is_zero_if_pre_dispatch_fee_is_zero() {
	new_test_ext().execute_with(|| {
		let tip = 0;
		let len = 10;
		let asset_id = 1_u128;
		let min_balance = 2;
		let balance = 10000 * UNIT_BALANCE;

		assert_ok!(AssetsTransactionPayment::allow_list_token_for_fees(Origin::root(), asset_id));

		assert_ok!(Balances::set_balance(Origin::root(), get_alice_account(), balance, 0));

		//create asset
		assert_ok!(Assets::force_create(
			Origin::root(),
			asset_id.into(),
			get_alice_account(), /* owner */
			true,                /* is_sufficient */
			min_balance
		));

		assert_ok!(Assets::mint(
			Origin::signed(get_alice_account().clone().into()),
			asset_id.into(),
			get_alice_account(),
			balance
		));

		let call: &<Test as frame_system::Config>::Call = &Call::Balances(BalancesCall::transfer {
			dest: get_alice_account(),
			value: 1.saturated_into(),
		});

		let charge_asset_transaction_payment =
			ChargeAssetTransactionPayment::<Test> { signature_scheme: 0, asset_id, tip };

		let pre_dispatch_result = charge_asset_transaction_payment
			.pre_dispatch(&get_alice_account(), call, &info_from_pays(Pays::No), len)
			.unwrap();
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance);

		assert!(ChargeAssetTransactionPayment::post_dispatch(
			Some(pre_dispatch_result),
			&info_from_pays(Pays::No),
			&post_info_from_pays(Pays::Yes),
			len,
			&Ok(())
		)
		.is_err());
		assert_eq!(Assets::balance(asset_id.into(), get_alice_account()), balance);
	});
}
