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

use crate::*;
use frame_support::{assert_noop, assert_ok};
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use frame_system::EventRecord;
use pallet_balances::BalanceLock;
use polkadex_primitives::{AccountId, UNIT_BALANCE};
use sp_runtime::{AccountId32, DispatchError::BadOrigin, WeakBoundedVec};
pub const STACK_SIZE: usize = 8388608;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
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
	(AccountId::new(NEAL_ACCOUNT_RAW_ID), 300 * UNIT_BALANCE)
}

fn get_rewards_claimable_at_start_block() -> (u128, u128, u128) {
	(50 * UNIT_BALANCE, 100 * UNIT_BALANCE, 150 * UNIT_BALANCE)
}

fn get_rewards_when_50_percentage_of_lock_amount_claimable() -> (u128, u128, u128) {
	(155 * UNIT_BALANCE, 310 * UNIT_BALANCE, 465 * UNIT_BALANCE)
}

fn get_rewards_when_75_percentage_of_lock_amount_claimable() -> (u128, u128, u128) {
	(
		162 * UNIT_BALANCE + 5_000_000_000_00,
		325 * UNIT_BALANCE,
		487 * UNIT_BALANCE + 5_000_000_000_00,
	)
}

//it returns a tuple (start_block ,end_block, initial_percentage, reward_id)
fn get_parameters_for_reward_cycle() -> (u64, u64, u32, u32) {
	(20, 120, 25, 1)
}

fn get_conversion_factor() -> u128 {
	2 * UNIT_BALANCE
}

fn amount_to_be_added_in_pallet_account(beneficiaries: Vec<(AccountId32, u128)>) -> u128 {
	//initial balance for paying fees
	let mut total_rewards_in_pdex = 10 * UNIT_BALANCE;
	for item in beneficiaries.clone().into_iter() {
		total_rewards_in_pdex +=
			item.1.saturating_mul(get_conversion_factor()).saturating_div(UNIT_BALANCE);
	}
	total_rewards_in_pdex
}

fn add_existential_deposit() {
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		get_alice_account_with_rewards().0,
		1 * UNIT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		get_neal_account_with_rewards().0,
		1 * UNIT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		get_bob_account_with_rewards().0,
		1 * UNIT_BALANCE,
		0
	));
}

#[test]
fn create_reward_cycle() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));
		assert_last_event::<Test>(
			crate::Event::RewardCycleCreated { start_block, end_block, reward_id }.into(),
		);
		let reward_info = InitializeRewards::<Test>::get(&reward_id).unwrap();
		assert_eq!(reward_info.start_block, start_block);
		assert_eq!(reward_info.end_block, end_block);
		assert_eq!(reward_info.initial_percentage, initial_percentage);
	});
}

#[test]
fn create_reward_cycle_with_invalid_root() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				RuntimeOrigin::none(),
				start_block,
				end_block,
				initial_percentage,
				reward_id
			),
			BadOrigin
		);
		assert_eq!(InitializeRewards::<Test>::get(&reward_id), None)
	});
}

#[test]
fn create_reward_cycle_for_existing_id() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));
		assert_noop!(
			Rewards::create_reward_cycle(
				RuntimeOrigin::root(),
				start_block,
				end_block,
				initial_percentage,
				reward_id
			),
			Error::<Test>::DuplicateId
		);
	});
}

#[test]
fn create_reward_cycle_when_start_block_greater_than_end_block() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				RuntimeOrigin::root(),
				end_block,
				start_block,
				initial_percentage,
				reward_id
			),
			Error::<Test>::InvalidBlocksRange
		);
	});
}

#[test]
fn create_reward_cycle_when_percentage_parameter_is_invalid() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::create_reward_cycle(
				RuntimeOrigin::root(),
				start_block,
				end_block,
				101,
				reward_id
			),
			Error::<Test>::InvalidInitialPercentage
		);
		assert_noop!(
			Rewards::create_reward_cycle(
				RuntimeOrigin::root(),
				start_block,
				end_block,
				0,
				reward_id
			),
			Error::<Test>::InvalidInitialPercentage
		);
	});
}

#[test]
fn initialize_claim_rewards() {
	//set stack size as 8 MB
	std::thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(|| {
			new_test_ext().execute_with(|| {
				let (start_block, end_block, initial_percentage, reward_id) =
					get_parameters_for_reward_cycle();
				let (mut alice_account, _) = get_alice_account_with_rewards();
				//get alice account from hashmap
				if let Some((key, _)) = crowdloan_rewardees::HASHMAP.iter().next() {
					alice_account = key.clone();
				}
				assert_ok!(Rewards::create_reward_cycle(
					RuntimeOrigin::root(),
					start_block,
					end_block,
					initial_percentage,
					reward_id
				));

				//add reward beneficiaries as alice and bob
				let beneficiaries: Vec<(AccountId32, u128)> =
					vec![get_alice_account_with_rewards()];

				let pallet_id_account = Rewards::get_pallet_account();

				//calculate total rewards in pdex
				let total_rewards_in_pdex =
					amount_to_be_added_in_pallet_account(beneficiaries.clone());

				//transfer balance to pallet account
				assert_ok!(Balances::set_balance(
					RuntimeOrigin::root(),
					pallet_id_account.clone(),
					total_rewards_in_pdex,
					0
				));

				assert_eq!(Balances::free_balance(&pallet_id_account), total_rewards_in_pdex);

				//alice needs to have Existential Deposit
				add_existential_deposit();

				System::set_block_number(start_block);

				// unlock alice reward
				assert_ok!(Rewards::initialize_claim_rewards(
					RuntimeOrigin::signed(alice_account.clone()),
					reward_id
				));

				//try to unlock reward for alice again
				assert_noop!(
					Rewards::initialize_claim_rewards(
						RuntimeOrigin::signed(alice_account.clone()),
						reward_id
					),
					Error::<Test>::RewardsAlreadyInitialized
				);

				let alice_reward_info =
					Distributor::<Test>::get(&reward_id, &alice_account.clone()).unwrap();
				assert_eq!(alice_reward_info.claim_amount, 0);
				assert_eq!(alice_reward_info.last_block_rewards_claim, start_block);
				assert_eq!(alice_reward_info.is_initial_rewards_claimed, false);
				assert_eq!(alice_reward_info.is_initialized, true);
				assert_eq!(alice_reward_info.lock_id, REWARDS_LOCK_ID);

				//assert event
				assert_last_event::<Test>(
					crate::Event::UserUnlockedReward { user: alice_account.clone(), reward_id }
						.into(),
				);

				let balance_locks: WeakBoundedVec<BalanceLock<u128>, MaxLocks> =
					Balances::locks(&alice_account);

				for lock in balance_locks.into_iter() {
					if lock.id == REWARDS_LOCK_ID {
						assert_eq!(lock.amount, 10274080000000_u128.saturated_into());
					} else {
						panic!("Invalid lock id");
					}
				}
			});
		})
		.unwrap()
		.join()
		.unwrap();
}

#[test]
fn initialize_claim_rewards_when_vesting_period_not_started() {
	//set stack size as 8 MB
	std::thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(|| {
			new_test_ext().execute_with(|| {
				let (start_block, end_block, initial_percentage, reward_id) =
					get_parameters_for_reward_cycle();

				assert_ok!(Rewards::create_reward_cycle(
					RuntimeOrigin::root(),
					start_block,
					end_block,
					initial_percentage,
					reward_id
				));

				//add reward beneficiaries as alice and bob
				let beneficiaries: Vec<(AccountId32, u128)> =
					vec![get_alice_account_with_rewards()];

				let pallet_id_account = Rewards::get_pallet_account();

				//calculate total rewards in pdex
				let total_rewards_in_pdex =
					amount_to_be_added_in_pallet_account(beneficiaries.clone());

				//transfer balance to pallet account
				assert_ok!(Balances::set_balance(
					RuntimeOrigin::root(),
					pallet_id_account.clone(),
					total_rewards_in_pdex,
					0
				));

				assert_eq!(Balances::free_balance(&pallet_id_account), total_rewards_in_pdex);

				//alice bob neal need to have Existential Deposit
				add_existential_deposit();

				System::set_block_number(start_block - 1);

				// unlock alice reward when vesting period not started
				assert_noop!(
					Rewards::initialize_claim_rewards(
						RuntimeOrigin::signed(get_alice_account_with_rewards().0.into()),
						reward_id
					),
					Error::<Test>::RewardsCannotBeUnlockYet
				);
			});
		})
		.unwrap()
		.join()
		.unwrap();
}

#[test]
fn initialize_claim_rewards_bad_origin() {
	//set stack size as 8 MB
	std::thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(|| {
			new_test_ext().execute_with(|| {
				let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
				assert_noop!(
					Rewards::initialize_claim_rewards(RuntimeOrigin::root(), reward_id),
					BadOrigin
				);
				assert_noop!(
					Rewards::initialize_claim_rewards(RuntimeOrigin::none(), reward_id),
					BadOrigin
				);
			});
		})
		.unwrap()
		.join()
		.unwrap();
}

#[test]
fn initialize_claim_rewards_with_non_existing_reward_id() {
	//set stack size as 8 MB
	std::thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(|| {
			new_test_ext().execute_with(|| {
				let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
				let (alice_account, _) = get_alice_account_with_rewards();
				assert_noop!(
					Rewards::initialize_claim_rewards(
						RuntimeOrigin::signed(alice_account.clone().into()),
						reward_id
					),
					Error::<Test>::RewardIdNotRegister
				);
			});
		})
		.unwrap()
		.join()
		.unwrap();
}

#[test]
fn initialize_claim_rewards_when_user_not_eligible_to_unlock() {
	//set stack size as 8 MB
	std::thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(|| {
			new_test_ext().execute_with(|| {
				let (start_block, end_block, initial_percentage, reward_id) =
					get_parameters_for_reward_cycle();
				assert_ok!(Rewards::create_reward_cycle(
					RuntimeOrigin::root(),
					start_block,
					end_block,
					initial_percentage,
					reward_id
				));
				let (bob_account, _) = get_bob_account_with_rewards();
				System::set_block_number(start_block);
				assert_noop!(
					Rewards::initialize_claim_rewards(
						RuntimeOrigin::signed(bob_account.clone().into()),
						reward_id
					),
					Error::<Test>::UserNotEligible
				);
			});
		})
		.unwrap()
		.join()
		.unwrap();
}

#[test]
pub fn claim_reward_for_bad_origin() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(Rewards::claim(RuntimeOrigin::root(), reward_id), BadOrigin);
	});
}

#[test]
pub fn claim_reward_for_unregister_id() {
	new_test_ext().execute_with(|| {
		let (_, _, _, reward_id) = get_parameters_for_reward_cycle();
		assert_noop!(
			Rewards::claim(
				RuntimeOrigin::signed(get_alice_account_with_rewards().0.into()),
				reward_id
			),
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
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));
		let (alice_account, _) = get_alice_account_with_rewards();

		assert_noop!(
			Rewards::claim(RuntimeOrigin::signed(alice_account.clone().into()), reward_id),
			Error::<Test>::UserNotEligible
		);
	});
}

fn assert_locked_balance(user: &AccountId, reward_claimable: u128, total_reward: u128) {
	let balance_locks: WeakBoundedVec<BalanceLock<u128>, MaxLocks> = Balances::locks(user);
	for lock in balance_locks.clone().into_iter() {
		if lock.id == REWARDS_LOCK_ID {
			assert_eq!(lock.amount, total_reward.saturating_sub(reward_claimable));
		} else {
			panic!("Reward id not present");
		}
	}
}

pub fn insert_reward(
	account: AccountId,
	total_reward_amount: u128,
	claim_amount: u128,
	initial_rewards_claimable: u128,
	factor: u128,
) {
	let reward_info = RewardInfoForAccount {
		total_reward_amount: total_reward_amount.saturated_into(),
		claim_amount: claim_amount.saturated_into(),
		is_initial_rewards_claimed: false,
		is_initialized: true,
		lock_id: REWARDS_LOCK_ID,
		last_block_rewards_claim: get_parameters_for_reward_cycle().0,
		initial_rewards_claimable: initial_rewards_claimable.saturated_into(),
		factor: factor.saturated_into(),
	};
	Distributor::<Test>::insert(&get_parameters_for_reward_cycle().3, account, reward_info);
}

/// For this test case initial percentage of rewards will be claimed.
#[test]
pub fn claim_rewards_at_start_block() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();
		let (bob_account, total_rewards_for_bob_in_dot) = get_bob_account_with_rewards();
		let (neal_account, total_rewards_for_neal_in_dot) = get_neal_account_with_rewards();

		let conversion_factor = get_conversion_factor();
		let total_reward_for_alice_in_pdex = total_rewards_for_alice_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_bob_in_pdex = total_rewards_for_bob_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_neal_in_pdex = total_rewards_for_neal_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let beneficiaries: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];

		insert_reward(
			get_alice_account_with_rewards().0,
			200000000000000_u128,
			0_u128,
			50000000000000_u128,
			1500000000000_u128,
		);
		insert_reward(
			get_neal_account_with_rewards().0,
			600000000000000,
			0_u128,
			150000000000000,
			4500000000000,
		);
		insert_reward(
			get_bob_account_with_rewards().0,
			400000000000000,
			0_u128,
			100000000000000,
			3000000000000,
		);

		let reward_info_for_alice =
			Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();

		assert_eq!(reward_info_for_alice.total_reward_amount, total_reward_for_alice_in_pdex);

		//add some existential deposit to alice, bob and neal
		add_existential_deposit();

		//calculate total rewards and set balance
		let total_rewards_in_pdex = amount_to_be_added_in_pallet_account(beneficiaries.clone());
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		System::set_block_number(start_block);

		let (alice_claimable, bob_claimable, neal_claimable) =
			get_rewards_claimable_at_start_block();

		//increment to the block at which the rewards are unlocked
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(bob_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(neal_account.clone()), reward_id));

		//assert locked balances
		assert_locked_balance(&alice_account, alice_claimable, total_reward_for_alice_in_pdex);
		assert_locked_balance(&bob_account, bob_claimable, total_reward_for_bob_in_pdex);
		assert_locked_balance(&neal_account, neal_claimable, total_reward_for_neal_in_pdex);
	})
}

/// For this test case 100 percentage of rewards will be claimed.
#[test]
pub fn claim_rewards_at_end_block() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();
		let (bob_account, total_rewards_for_bob_in_dot) = get_bob_account_with_rewards();
		let (neal_account, total_rewards_for_neal_in_dot) = get_neal_account_with_rewards();

		let conversion_factor = get_conversion_factor();
		let total_reward_for_alice_in_pdex = total_rewards_for_alice_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_bob_in_pdex = total_rewards_for_bob_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_neal_in_pdex = total_rewards_for_neal_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let beneficiaries: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];

		insert_reward(
			get_alice_account_with_rewards().0,
			200000000000000_u128,
			0_u128,
			50000000000000_u128,
			1500000000000_u128,
		);
		insert_reward(
			get_neal_account_with_rewards().0,
			600000000000000,
			0_u128,
			150000000000000,
			4500000000000,
		);
		insert_reward(
			get_bob_account_with_rewards().0,
			400000000000000,
			0_u128,
			100000000000000,
			3000000000000,
		);

		let reward_info_for_alice =
			Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();

		assert_eq!(reward_info_for_alice.total_reward_amount, total_reward_for_alice_in_pdex);

		//add some existential deposit to alice, bob and neal
		add_existential_deposit();

		//calculate total rewards and set balance
		let total_rewards_in_pdex = amount_to_be_added_in_pallet_account(beneficiaries.clone());
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		System::set_block_number(end_block);

		//increment to the block at which the rewards are unlocked
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(bob_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(neal_account.clone()), reward_id));

		//assert locked balances
		assert_locked_balance(
			&alice_account,
			total_reward_for_alice_in_pdex,
			total_reward_for_alice_in_pdex,
		);
		assert_locked_balance(
			&bob_account,
			total_reward_for_bob_in_pdex,
			total_reward_for_bob_in_pdex,
		);
		assert_locked_balance(
			&neal_account,
			total_reward_for_neal_in_pdex,
			total_reward_for_neal_in_pdex,
		);
	})
}

//// For this test case 50 percentage of locked rewards will be claimed.
#[test]
pub fn claim_rewards_at_50_percentage_of_reward_period() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();
		let (bob_account, total_rewards_for_bob_in_dot) = get_bob_account_with_rewards();
		let (neal_account, total_rewards_for_neal_in_dot) = get_neal_account_with_rewards();

		let conversion_factor = get_conversion_factor();
		let total_reward_for_alice_in_pdex = total_rewards_for_alice_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_bob_in_pdex = total_rewards_for_bob_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_neal_in_pdex = total_rewards_for_neal_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let beneficiaries: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];

		insert_reward(
			get_alice_account_with_rewards().0,
			200000000000000_u128,
			0_u128,
			50000000000000_u128,
			1500000000000_u128,
		);
		insert_reward(
			get_neal_account_with_rewards().0,
			600000000000000,
			0_u128,
			150000000000000,
			4500000000000,
		);
		insert_reward(
			get_bob_account_with_rewards().0,
			400000000000000,
			0_u128,
			100000000000000,
			3000000000000,
		);

		let reward_info_for_alice =
			Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();

		assert_eq!(reward_info_for_alice.total_reward_amount, total_reward_for_alice_in_pdex);

		//add some existential deposit to alice, bob and neal
		add_existential_deposit();

		//calculate total rewards and set balance
		let total_rewards_in_pdex = amount_to_be_added_in_pallet_account(beneficiaries.clone());
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		let require_block_to_claim_50_percentage_of_rewards =
			start_block.saturating_add(end_block).saturating_div(2);
		System::set_block_number(
			start_block.saturating_add(require_block_to_claim_50_percentage_of_rewards),
		);

		//increment to the block at which the rewards are unlocked
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(bob_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(neal_account.clone()), reward_id));

		let (alice_claimable, bob_claimable, neal_claimable) =
			get_rewards_when_50_percentage_of_lock_amount_claimable();

		//assert locked balances
		assert_locked_balance(&alice_account, alice_claimable, total_reward_for_alice_in_pdex);
		assert_locked_balance(&bob_account, bob_claimable, total_reward_for_bob_in_pdex);
		assert_locked_balance(&neal_account, neal_claimable, total_reward_for_neal_in_pdex);
	})
}

/// For this test case 75 percentage of locked rewards will be claimed.
#[test]
pub fn claim_rewards_at_75_percentage_of_reward_period() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();
		let (bob_account, total_rewards_for_bob_in_dot) = get_bob_account_with_rewards();
		let (neal_account, total_rewards_for_neal_in_dot) = get_neal_account_with_rewards();

		let conversion_factor = get_conversion_factor();
		let total_reward_for_alice_in_pdex = total_rewards_for_alice_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_bob_in_pdex = total_rewards_for_bob_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let total_reward_for_neal_in_pdex = total_rewards_for_neal_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let beneficiaries: Vec<(AccountId32, u128)> = vec![
			get_alice_account_with_rewards(),
			get_neal_account_with_rewards(),
			get_bob_account_with_rewards(),
		];

		insert_reward(
			get_alice_account_with_rewards().0,
			200000000000000_u128,
			0_u128,
			50000000000000_u128,
			1500000000000_u128,
		);
		insert_reward(
			get_neal_account_with_rewards().0,
			600000000000000,
			0_u128,
			150000000000000,
			4500000000000,
		);
		insert_reward(
			get_bob_account_with_rewards().0,
			400000000000000,
			0_u128,
			100000000000000,
			3000000000000,
		);

		let reward_info_for_alice =
			Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();

		assert_eq!(reward_info_for_alice.total_reward_amount, total_reward_for_alice_in_pdex);

		//add some existential deposit to alice, bob and neal
		add_existential_deposit();

		//calculate total rewards and set balance
		let total_rewards_in_pdex = amount_to_be_added_in_pallet_account(beneficiaries.clone());
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		let require_block_to_claim_75_percentage_of_rewards = 95;
		System::set_block_number(require_block_to_claim_75_percentage_of_rewards);

		//increment to the block at which the rewards are unlocked
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(bob_account.clone()), reward_id));
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(neal_account.clone()), reward_id));

		let (alice_claimable, bob_claimable, neal_claimable) =
			get_rewards_when_75_percentage_of_lock_amount_claimable();

		//assert locked balances
		assert_locked_balance(&alice_account, alice_claimable, total_reward_for_alice_in_pdex);
		assert_locked_balance(&bob_account, bob_claimable, total_reward_for_bob_in_pdex);
		assert_locked_balance(&neal_account, neal_claimable, total_reward_for_neal_in_pdex);
	})
}

#[test]
pub fn claim_rewards_for_alice_at_multiple_intervals() {
	new_test_ext().execute_with(|| {
		let (start_block, end_block, initial_percentage, reward_id) =
			get_parameters_for_reward_cycle();
		//create reward cycle
		assert_ok!(Rewards::create_reward_cycle(
			RuntimeOrigin::root(),
			start_block,
			end_block,
			initial_percentage,
			reward_id
		));

		//add beneficiaries
		let (alice_account, total_rewards_for_alice_in_dot) = get_alice_account_with_rewards();

		let conversion_factor = get_conversion_factor();
		let total_reward_for_alice_in_pdex = total_rewards_for_alice_in_dot
			.saturating_mul(conversion_factor)
			.saturating_div(UNIT_BALANCE);

		let beneficiaries: Vec<(AccountId32, u128)> = vec![get_alice_account_with_rewards()];

		insert_reward(
			get_alice_account_with_rewards().0,
			200000000000000_u128,
			0_u128,
			50000000000000_u128,
			1500000000000_u128,
		);
		insert_reward(
			get_neal_account_with_rewards().0,
			600000000000000,
			0_u128,
			150000000000000,
			4500000000000,
		);
		insert_reward(
			get_bob_account_with_rewards().0,
			400000000000000,
			0_u128,
			100000000000000,
			3000000000000,
		);

		let reward_info_for_alice =
			Rewards::get_account_reward_info(reward_id, &alice_account).unwrap();

		assert_eq!(reward_info_for_alice.total_reward_amount, total_reward_for_alice_in_pdex);

		//add some existential deposit to alice
		add_existential_deposit();

		//calculate total rewards and set balance
		let total_rewards_in_pdex = amount_to_be_added_in_pallet_account(beneficiaries.clone());
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			Rewards::get_pallet_account(),
			total_rewards_in_pdex,
			0
		));

		let block_number = start_block;
		System::set_block_number(block_number);

		//increment to the block at which the rewards are unlocked
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		let (alice_claimable, _, _) = get_rewards_claimable_at_start_block();

		assert_locked_balance(&alice_account, alice_claimable, total_reward_for_alice_in_pdex);

		//re try to call claim at a block at which 50% of lock rewards can be claimed
		let require_block_to_claim_50_percentage_of_rewards =
			start_block.saturating_add(end_block).saturating_div(2);
		System::set_block_number(
			start_block.saturating_add(require_block_to_claim_50_percentage_of_rewards),
		);

		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		let (alice_claimable, _, _) = get_rewards_when_50_percentage_of_lock_amount_claimable();

		//assert locked balances
		assert_locked_balance(&alice_account, alice_claimable, total_reward_for_alice_in_pdex);

		//call claim at the end of cycle
		System::set_block_number(end_block + 10);
		assert_ok!(Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id));
		//assert locked balances
		assert_locked_balance(
			&alice_account,
			total_reward_for_alice_in_pdex,
			total_reward_for_alice_in_pdex,
		);

		//re try to call claim at the end of cycle when all rewards claimed
		System::set_block_number(end_block + 20);
		assert_noop!(
			Rewards::claim(RuntimeOrigin::signed(alice_account.clone()), reward_id),
			Error::<Test>::AmountToLowToRedeem
		);
	})
}
