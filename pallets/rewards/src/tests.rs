use crate::*;
use frame_support::{assert_noop, assert_ok};
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use frame_system::EventRecord;
use polkadex_primitives::AccountId;
use sp_runtime::{AccountId32, BoundedVec, DispatchError::BadOrigin};
use std::convert::TryFrom;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

//Bob main account id
pub const BOB_ACCOUNT_RAW_ID: [u8; 32] = [6u8; 32];
//Bob proxy account id
pub const ALICE_ACCOUNT_RAW_ID: [u8; 32] = [7u8; 32];

fn get_alice_account_with_rewards() -> (AccountId32, u128) {
	(AccountId::new(ALICE_ACCOUNT_RAW_ID), 100)
}

fn get_bob_account_with_rewards() -> (AccountId32, u128) {
	(AccountId::new(BOB_ACCOUNT_RAW_ID), 200)
}

fn get_parameters_for_reward_cycle() -> (u32, u32, u32, u32) {
	(2, 5, 10, 1)
}

#[test]
fn create_reward_cycle() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, id) = get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			id
		));
		assert_last_event::<Test>(
			crate::Event::RewardCycleCreated { start_block, end_block, id }.into(),
		);
		let reward_info = IntializeRewards::<Test>::get(&id).unwrap();
		assert_eq!(reward_info.start_block, start_block);
		assert_eq!(reward_info.end_block, end_block);
		assert_eq!(reward_info.intial_percentage, intial_percentage);
	});
}

#[test]
fn create_reward_cycle_with_invalid_root() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::none(),
				start_block,
				end_block,
				intial_percentage,
				id
			),
			BadOrigin
		);
		assert_eq!(IntializeRewards::<Test>::get(&id), None)
	});
}

#[test]
fn create_reward_cycle_for_existing_id() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, id) = get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			id
		));
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::root(),
				start_block,
				end_block,
				intial_percentage,
				id
			),
			Error::<Test>::DuplicateId
		);
	});
}

#[test]
fn create_reward_cycle_when_start_block_greater_than_end_block() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				Origin::root(),
				end_block,
				start_block,
				intial_percentage,
				id
			),
			Error::<Test>::InvalidParameter
		);
	});
}

#[test]
fn add_reward_beneficiaries_with_invalid_root() {
	new_test_ext().execute_with(|| {
		let (_, _, _, id) = get_parameters_for_reward_cycle();
		let vec_of_ids: Vec<(AccountId32, u128)> = vec![];
		assert_noop!(
			Rewards::add_reward_beneficiaries(
				Origin::none(),
				id,
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
		let vec_of_ids: Vec<(AccountId32, u128)> = vec![];
		assert_noop!(
			Rewards::add_reward_beneficiaries(
				Origin::root(),
				id,
				BoundedVec::try_from(vec_of_ids).unwrap()
			),
			Error::<Test>::RewardIdNotRegister
		);
	});
}

#[test]
fn add_reward_beneficiaries() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, intial_percentage, id) = get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			Origin::root(),
			start_block,
			end_block,
			intial_percentage,
			id
		));
		let vec_of_ids: Vec<(AccountId32, u128)> =
			vec![get_alice_account_with_rewards(), get_bob_account_with_rewards()];
		assert_ok!(Rewards::add_reward_beneficiaries(
			Origin::root(),
			id,
			BoundedVec::try_from(vec_of_ids).unwrap()
		));

		let alice_reward_info =
			Distributor::<Test>::get(&id, &get_alice_account_with_rewards().0).unwrap();
		assert_eq!(alice_reward_info.total_amount, get_alice_account_with_rewards().1);
		assert_eq!(alice_reward_info.claim_amount, 0);
		assert_eq!(alice_reward_info.staked_amount, 0);
		assert_eq!(alice_reward_info.last_block_rewards_claim, 0);

		let bob_reward_info =
			Distributor::<Test>::get(&id, &get_bob_account_with_rewards().0).unwrap();
		assert_eq!(bob_reward_info.total_amount, get_bob_account_with_rewards().1);
		assert_eq!(bob_reward_info.claim_amount, 0);
		assert_eq!(bob_reward_info.staked_amount, 0);
		assert_eq!(bob_reward_info.last_block_rewards_claim, 0);
	});
}
