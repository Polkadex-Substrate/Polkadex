use crate::*;
use frame_support::{assert_noop, assert_ok};
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use frame_system::EventRecord;
use polkadex_primitives::AccountId;
use sp_runtime::{AccountId32, DispatchError::BadOrigin, WeakBoundedVec};
use std::convert::TryFrom;
use pallet_balances::BalanceLock;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

//Bob account id
pub const BOB_ACCOUNT_RAW_ID: [u8; 32] = [6u8; 32];
//Bob account id
pub const ALICE_ACCOUNT_RAW_ID: [u8; 32] = [7u8; 32];
//Neal account id
pub const NEAL_ACCOUNT_RAW_ID: [u8; 32] = [5u8; 32];

fn get_alice_account_with_rewards() -> (AccountId32, u128) {
	(AccountId::new(ALICE_ACCOUNT_RAW_ID), 100 * UNIT_BALANCE)
}

fn get_bob_account_with_rewards() -> (AccountId32, u128) {
	(AccountId::new(BOB_ACCOUNT_RAW_ID), 200 * UNIT_BALANCE)
}

fn get_neal_account_with_rewards() -> (AccountId32, u128) {
	(AccountId::new(NEAL_ACCOUNT_RAW_ID), 3 * UNIT_BALANCE)
}

fn get_neal_account_with_invalid_rewards() -> (AccountId32, u128) {
	(AccountId::new(NEAL_ACCOUNT_RAW_ID), 1_000_000_000)
}

fn get_parameters_for_reward_cycle() -> (u64, u64, u32, u32) {
	(20, 50, 10, 1)
}

fn get_conversion_factor() -> u128 {
	2 * UNIT_BALANCE
}

#[test]
fn create_reward_cycle() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));
		assert_last_event::<Test>(
			crate::Event::RewardCycleCreated { start_block, end_block, reward_id }.into(),
		);
		let reward_info = IntializeRewards::<Test>::get(&reward_id).unwrap();
		assert_eq!(reward_info.start_block, start_block);
		assert_eq!(reward_info.end_block, end_block);
		assert_eq!(reward_info.intial_percentage, intial_percentage);
	});
}

#[test]
fn create_reward_cycle_with_invalid_root() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::none(),
				start_block,
				end_block,
				intial_percentage,
				reward_id
			),
			BadOrigin
		);
		assert_eq!(IntializeRewards::<Test>::get(&reward_id), None)
	});
}

#[test]
fn create_reward_cycle_for_existing_id() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::root(),
				start_block,
				end_block,
				intial_percentage,
				reward_id
			),
			Error::<Test>::DuplicateId
		);
	});
}

#[test]
fn create_reward_cycle_when_start_block_greater_than_end_block() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::root(),
				end_block,
				start_block,
				intial_percentage,
				reward_id
			),
			Error::<Test>::InvalidParameter
		);
	});
}

#[test]
fn create_reward_cycle_when_percentage_parameter_is_invalid() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(Origin::root(), end_block, start_block, 101, reward_id),
			Error::<Test>::InvalidParameter
		);
	});
}
//- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//-   - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn add_reward_beneficiaries() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();

		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));

		//add reward beneficiaries as alice and bob
		let vec_of_ids: Vec<(AccountId32, u128)> =
			vec![get_alice_account_with_rewards(), get_bob_account_with_rewards()];
		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			reward_id,
			conversion_factor,
			BoundedVec::try_from(vec_of_ids).unwrap()
		));

		let alice_reward_info =
			Distributor::<Test>::get(&reward_id, &get_alice_account_with_rewards().0).unwrap();

		assert_eq!(
			alice_reward_info.total_reward_amount,
			get_alice_account_with_rewards()
				.1
				.saturating_mul(conversion_factor)
				.saturating_div(UNIT_BALANCE)
		);
		assert_eq!(alice_reward_info.claim_amount, 0);
		assert_eq!(alice_reward_info.last_block_rewards_claim, start_block);
		assert_eq!(alice_reward_info.is_intial_rewards_claimed, false);
		assert_eq!(alice_reward_info.is_intialized, false);
		assert_eq!(alice_reward_info.lock_id, REWARDS_LOCK_ID);

		let bob_reward_info =
			Distributor::<Test>::get(&reward_id, &get_bob_account_with_rewards().0).unwrap();

		assert_eq!(
			bob_reward_info.total_reward_amount,
			get_bob_account_with_rewards()
				.1
				.saturating_mul(conversion_factor)
				.saturating_div(UNIT_BALANCE)
		);
		assert_eq!(bob_reward_info.claim_amount, 0);
		assert_eq!(bob_reward_info.last_block_rewards_claim, start_block);
		assert_eq!(bob_reward_info.is_intial_rewards_claimed, false);
		assert_eq!(bob_reward_info.is_intialized, false);
		assert_eq!(bob_reward_info.lock_id, REWARDS_LOCK_ID);
	});
}

#[test]
fn add_reward_beneficiaries_with_invalid_root() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();
		let vec_of_ids: Vec<(AccountId32, u128)> = vec![];
		assert_noop!(
			Rewards::add_reward_beneficiaries(
				Origin::none(),
				reward_id,
				conversion_factor,
				BoundedVec::try_from(vec_of_ids).unwrap()
			),
			BadOrigin
		);
	});
}

#[test]
fn add_reward_beneficiaries_when_reward_id_not_register() {
	new_test_ext().execute_with(|| {
		let (_, _, _, id) = get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();

		let vec_of_ids: Vec<(AccountId32, u128)> = vec![];
		assert_noop!(
			Rewards::add_reward_beneficiaries(
				Origin::root(),
				id,
				conversion_factor,
				BoundedVec::try_from(vec_of_ids).unwrap()
			),
			Error::<Test>::RewardIdNotRegister
		);
	});
}
//
#[test]
fn add_one_beneficiary_which_falls_below_threshold() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();

		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));

		//add reward beneficiaries as alice and bob
		let vec_of_ids: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_bob_account_with_rewards(),
			get_neal_account_with_invalid_rewards(),
		];
		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			reward_id,
			conversion_factor,
			BoundedVec::try_from(vec_of_ids).unwrap()
		));

		let alice_reward_info =
			Distributor::<Test>::get(&reward_id, &get_alice_account_with_rewards().0).unwrap();

		assert_eq!(
			alice_reward_info.total_reward_amount,
			get_alice_account_with_rewards()
				.1
				.saturating_mul(conversion_factor)
				.saturating_div(UNIT_BALANCE)
		);
		assert_eq!(alice_reward_info.claim_amount, 0);
		assert_eq!(alice_reward_info.last_block_rewards_claim, start_block);
		assert_eq!(alice_reward_info.is_intial_rewards_claimed, false);
		assert_eq!(alice_reward_info.is_intialized, false);
		assert_eq!(alice_reward_info.lock_id, REWARDS_LOCK_ID);

		let bob_reward_info =
			Distributor::<Test>::get(&reward_id, &get_bob_account_with_rewards().0).unwrap();

		assert_eq!(
			bob_reward_info.total_reward_amount,
			get_bob_account_with_rewards()
				.1
				.saturating_mul(conversion_factor)
				.saturating_div(UNIT_BALANCE)
		);
		assert_eq!(bob_reward_info.claim_amount, 0);
		assert_eq!(bob_reward_info.last_block_rewards_claim, start_block);
		assert_eq!(bob_reward_info.is_intial_rewards_claimed, false);
		assert_eq!(bob_reward_info.is_intialized, false);
		assert_eq!(bob_reward_info.lock_id, REWARDS_LOCK_ID);

		let neal_reward_info =
			Distributor::<Test>::get(&reward_id, &get_bob_account_with_rewards().0).unwrap();

		assert_eq!(
			Distributor::<Test>::get(&reward_id, &get_neal_account_with_invalid_rewards().0),
			None
		);

		assert_last_event::<Test>(
			crate::Event::UserRewardNotSatisfyingMinConstraint {
				user: get_neal_account_with_invalid_rewards().0,
				amount_in_pdex: get_neal_account_with_invalid_rewards()
					.1
					.saturating_mul(conversion_factor)
					.saturating_div(UNIT_BALANCE)
					.saturated_into(),
				reward_id,
			}
			.into(),
		);
	});
}
//- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//-   - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn unlock_rewards_for_alice() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();
		let (alice_account, _) = get_alice_account_with_rewards();

		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));

		//add reward beneficiaries as alice and bob
		let vec_of_ids: Vec<(AccountId32, u128)> =
			vec![get_alice_account_with_rewards(), get_bob_account_with_rewards()];
		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			reward_id,
			conversion_factor,
			BoundedVec::try_from(vec_of_ids).unwrap()
		));

		let pallet_id_account = Rewards::get_pallet_account();

		//transfer balance to pallet account
		assert_ok!(Balances::set_balance(
			Origin::root(),
			pallet_id_account.clone(),
			2000 * UNIT_BALANCE,
			0
		));

		assert_eq!(Balances::free_balance(&pallet_id_account), 2000 * UNIT_BALANCE);

		//need to have Existential Deposit
		assert_ok!(Balances::set_balance(
			Origin::root(),
			alice_account.clone(),
			2 * UNIT_BALANCE,
			0
		));

		// unlock alice reward
		assert_ok!(Rewards::unlock_reward(
			Origin::signed(get_alice_account_with_rewards().0.into()),
			reward_id
		));

		let alice_reward_info =
			Distributor::<Test>::get(&reward_id, &get_alice_account_with_rewards().0).unwrap();
		assert_eq!(alice_reward_info.claim_amount, 0);
		assert_eq!(alice_reward_info.last_block_rewards_claim, start_block);
		assert_eq!(alice_reward_info.is_intial_rewards_claimed, false);
		assert_eq!(alice_reward_info.is_intialized, true);
		assert_eq!(alice_reward_info.lock_id, REWARDS_LOCK_ID);

		//assert event
		assert_last_event::<Test>(
			crate::Event::UserUnlockedReward {
				user: get_alice_account_with_rewards().0,
				reward_id,
			}
			.into(),
		);

		let balance_locks: WeakBoundedVec<BalanceLock<u128>, MaxLocks> =
			Balances::locks(&alice_account);

		for lock in balance_locks.into_iter() {
			if lock.id == REWARDS_LOCK_ID {
				assert_eq!(lock.amount, 200 * UNIT_BALANCE);
			} else {
				panic!("Invalid lock id");
			}
		}
	});
}

#[test]
fn unlock_rewards_bad_origin() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(Rewards::unlock_reward(Origin::root(), reward_id), BadOrigin);
		assert_noop!(Rewards::unlock_reward(Origin::none(), reward_id), BadOrigin);
	});
}

#[test]
fn unlock_rewards_with_non_existing_reward_id() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		let (alice_account, _) = get_alice_account_with_rewards();
		assert_noop!(
			Rewards::unlock_reward(Origin::signed(alice_account.clone().into()), reward_id),
			Error::<Test>::RewardIdNotRegister
		);
	});
}

#[test]
fn unlock_rewards_when_user_not_eligible_to_unlock() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));
		let (alice_account, _) = get_alice_account_with_rewards();

		assert_noop!(
			Rewards::unlock_reward(Origin::signed(alice_account.clone().into()), reward_id),
			Error::<Test>::UserNotEligible
		);
	});
}
//- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//-   - - - - - - - - - - - - - - - - - - - - - - -

#[test]
pub fn claim_rewards() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		let conversion_factor = get_conversion_factor();

		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			reward_id
		));

		//add reward beneficiaries as alice and bob
		let vec_of_ids: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];
		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			reward_id,
			conversion_factor,
			BoundedVec::try_from(vec_of_ids).unwrap()
		));

		let pallet_id_account = Rewards::get_pallet_account();

		//transfer balance to pallet account
		assert_ok!(Balances::set_balance(
			Origin::root(),
			pallet_id_account.clone(),
			2000000000000000000,
			0
		));

		//need to have Existential Deposit for alice bob neal
		assert_ok!(Rewards::transfer_pdex_rewards(
			&pallet_id_account,
			&get_alice_account_with_rewards().0,
			1000000000000_u128.saturated_into()
		));
		assert_ok!(Rewards::transfer_pdex_rewards(
			&pallet_id_account,
			&get_bob_account_with_rewards().0,
			1000000000000_u128.saturated_into()
		));
		assert_ok!(Rewards::transfer_pdex_rewards(
			&pallet_id_account,
			&get_neal_account_with_rewards().0,
			1000000000000_u128.saturated_into()
		));

		//unlock rewards for alice bob neal
		assert_ok!(Rewards::unlock_reward(
			Origin::signed(get_alice_account_with_rewards().0.into()),
			reward_id
		));
		assert_ok!(Rewards::unlock_reward(
			Origin::signed(get_bob_account_with_rewards().0.into()),
			reward_id
		));
		assert_ok!(Rewards::unlock_reward(
			Origin::signed(get_neal_account_with_rewards().0.into()),
			reward_id
		));

		System::set_block_number(5);
		// claim rewards for alice
		assert_ok!(Rewards::claim(
			Origin::signed(get_alice_account_with_rewards().0.into()),
			reward_id
		));
	});
}

#[test]
pub fn claim_reward_for_bad_origin() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(Rewards::claim(Origin::root(), reward_id), BadOrigin);
	});
}

#[test]
pub fn claim_reward_for_unregister_id() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::claim(Origin::signed(get_alice_account_with_rewards().0.into()), reward_id),
			Error::<Test>::RewardIdNotRegister
		);
	});
}

#[test]
pub fn claim_reward_when_user_not_eligible() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));
		let (alice_account, _) = get_alice_account_with_rewards();

		assert_noop!(
			Rewards::claim(Origin::signed(alice_account.clone().into()), reward_id),
			Error::<Test>::UserNotEligible
		);
	});
}
#[test]
pub fn claim_reward_after_user_initialized_unlock() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();
		let conversion_factor = get_conversion_factor();
		let total__for_alice_rewards_in_pdex = total_rewards_for_alice_in_dot.saturating_mul(conversion_factor).saturating_div(UNIT_BALANCE);
		let beneficiaries: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];

		//calculate total rewards
		let mut total_rewards_in_pdex = 0;
		for item in beneficiaries.clone().into_iter() {
			total_rewards_in_pdex += item.1.saturating_mul(conversion_factor).saturating_div(UNIT_BALANCE);
		}

		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			reward_id,
			conversion_factor,
			BoundedVec::try_from(beneficiaries).unwrap()
		));
		let reward_info_for_alice = Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();
		assert_eq!(reward_info_for_alice.total_reward_amount, total__for_alice_rewards_in_pdex);
		assert_ok!(Balances::set_balance(
			Origin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		//add some existential balance to alice
		assert_ok!(Balances::set_balance(
			Origin::root(),
			alice_account.clone(),
			2 * UNIT_BALANCE,
			0
		));
		assert_eq!(Balances::free_balance(&alice_account), 2 * UNIT_BALANCE);
		System::set_block_number(start_block+1);
		assert_ok!(Rewards::unlock_reward(Origin::signed(alice_account.clone()), reward_id));
		//check locked balance
		//increment to the block at which the rewards are unlocked
		let balanceLocks: WeakBoundedVec<BalanceLock<u128>, MaxLocks> =
			Balances::locks(&alice_account);
		for lock in balanceLocks.into_iter() {
			if lock.id == REWARDS_LOCK_ID {
				assert_eq!(lock.amount, 2 * UNIT_BALANCE);
			}
		}
		assert_ok!(Rewards::claim(Origin::signed(alice_account.clone()), reward_id));
	})
}
//
// #[test]
// pub fn claim_reward_at_25_percent_cycle_of_reward_period(){
// 	let(start_block, end_block, initial_percentage, reward_id)= get_parameters_for_reward_cycle();
// 	assert_ok!(Rewards::create_reward_cycle(
// 			Origin::root(),
// 			start_block,
// 			end_block,
// 			initial_percentage,
// 			reward_id
// 		));
// 	let (alice_account, total_rewards) = get_alice_account_with_rewards();
// }
//
// #[test]
// pub fn claim_reward_at_50_percent_cycle_of_reward_period(){
// 	let(start_block, end_block, initial_percentage, reward_id)= get_parameters_for_reward_cycle();
// 	assert_ok!(Rewards::create_reward_cycle(
// 			Origin::root(),
// 			start_block,
// 			end_block,
// 			initial_percentage,
// 			reward_id
// 		));
// 	let (alice_account, total_rewards) = get_alice_account_with_rewards();
// }
//
// #[test]
// pub fn claim_reward_at_75_percent_of_reward_period(){
// 	let(start_block, end_block, initial_percentage, reward_id)= get_parameters_for_reward_cycle();
// 	assert_ok!(Rewards::create_reward_cycle(
// 			Origin::root(),
// 			start_block,
// 			end_block,
// 			initial_percentage,
// 			reward_id
// 		));
// 	let (alice_account, total_rewards) = get_alice_account_with_rewards();
// }
//
// #[test]
// pub fn claim_reward_at_100_percent_of_reward_period(){
// 	let(start_block, end_block, initial_percentage, reward_id)= get_parameters_for_reward_cycle();
// 	assert_ok!(Rewards::create_reward_cycle(
// 			Origin::root(),
// 			start_block,
// 			end_block,
// 			initial_percentage,
// 			reward_id
// 		));
// 	let (alice_account, total_rewards) = get_alice_account_with_rewards();
// }
//
// #[test]
// pub fn claim_reward_after_100_percent_of_reward_period(){
// 	let(start_block, end_block, initial_percentage, reward_id)= get_parameters_for_reward_cycle();
// 	assert_ok!(Rewards::create_reward_cycle(
// 			Origin::root(),
// 			start_block,
// 			end_block,
// 			initial_percentage,
// 			reward_id
// 		));
// 	let (alice_account, total_rewards) = get_alice_account_with_rewards();
// }
//- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//-   - - - - - - - - - - - - - - - - - - - - - - -
