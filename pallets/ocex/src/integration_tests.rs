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

//! Integration Tests for pallet-ocex.

use crate::aggregator::AggregatorClient;
use crate::mock::new_test_ext;
use crate::mock::*;
use crate::pallet::{Accounts, IngressMessages as IngressMessagesStorage, TradingPairs};
use crate::snapshot::StateInfo;
use crate::storage::{store_trie_root, OffchainState};
use crate::tests::get_trading_pair;
use crate::validator::{LAST_PROCESSED_SNAPSHOT, WORKER_STATUS};
use crate::Config;
use frame_support::assert_ok;
use frame_support::traits::fungible::Mutate;
use frame_support::traits::fungibles::Mutate as FunMutate;
use num_traits::{FromPrimitive, One};
use orderbook_primitives::constants::FEE_POT_PALLET_ID;
use orderbook_primitives::ingress::{EgressMessages, IngressMessages};
use orderbook_primitives::lmp::LMPMarketConfigWrapper;
use orderbook_primitives::ocex::{AccountInfo, TradingPairConfig};
use orderbook_primitives::types::UserActions;
use orderbook_primitives::types::{
	Order, OrderPayload, OrderSide, OrderStatus, OrderType, Trade, TradingPair, UserActionBatch,
};
use orderbook_primitives::SnapshotSummary;
use parity_scale_codec::{Compact, Encode};
use polkadex_primitives::auction::FeeDistribution;
use polkadex_primitives::{AccountId, AssetId, UNIT_BALANCE};
use rust_decimal::Decimal;
use sequential_test::sequential;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::AccountId32;
use sp_core::sr25519::Signature;
use sp_core::{Pair, H256};
use sp_runtime::offchain::storage::StorageValueRef;
use std::collections::BTreeMap;

#[test]
#[sequential]
fn test_run_on_chain_validation_trades_happy_path() {
	new_test_ext().execute_with(|| {
		push_trade_user_actions(1, 0, 1);
		assert_ok!(OCEX::run_on_chain_validation(1));
		let snapshot_id: u64 = 1;
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());
		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref
			.get::<(SnapshotSummary<AccountId32>, crate::sr25519::AuthoritySignature, u16)>()
		{
			Ok(Some((summary, signature, index))) => {
				assert_eq!(summary.snapshot_id, 1);
				assert_eq!(summary.state_change_id, 1);
				assert_eq!(summary.last_processed_blk, 1);
			},
			_ => panic!("Snapshot not found"),
		};
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);
		let mut state_info = match OCEX::load_state_info(&mut state) {
			Ok(info) => info,
			Err(err) => {
				log::error!(target:"ocex","Err loading state info from storage: {:?}",err);
				store_trie_root(H256::zero());
				panic!("Error {:?}", err);
			},
		};
		assert_eq!(state_info.last_block, 1);
		assert_eq!(state_info.stid, 1);
		assert_eq!(state_info.snapshot_id, 0);
	});
}

#[test]
#[sequential]
fn test_lmp_complete_flow() {
	new_test_ext().execute_with(|| {
		set_lmp_config();
		push_trade_user_actions(1, 1, 1);
		assert_ok!(OCEX::run_on_chain_validation(1));
		let snapshot_id: u64 = 1;
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());
		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref
			.get::<(SnapshotSummary<AccountId32>, crate::sr25519::AuthoritySignature, u16)>()
		{
			Ok(Some((summary, signature, index))) => {
				println!("Summary {:?}", summary);
				assert_eq!(summary.snapshot_id, 1);
				assert_eq!(summary.state_change_id, 1);
				assert_eq!(summary.last_processed_blk, 1);
				assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), summary, Vec::new()));
			},
			_ => panic!("Snapshot not found"),
		};
		OCEX::start_new_epoch(2);
		push_trade_user_actions(2, 1, 2);
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		s_info.set(&false);
		assert_ok!(OCEX::run_on_chain_validation(2));
		let snapshot_id: u64 = 2;
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());
		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref
			.get::<(SnapshotSummary<AccountId32>, crate::sr25519::AuthoritySignature, u16)>()
		{
			Ok(Some((summary, signature, index))) => {
				println!("Summary {:?}", summary);
				assert_eq!(summary.snapshot_id, 2);
				assert_eq!(summary.state_change_id, 2);
				assert_eq!(summary.last_processed_blk, 2);
				assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), summary, Vec::new()));
			},
			_ => panic!("Snapshot not found"),
		};
		OCEX::start_new_epoch(3);
		let (maker_account, taker_account) = get_maker_and_taker__account();
		let trading_pair = TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) };
		assert_ok!(OCEX::claim_lmp_rewards(
			RuntimeOrigin::signed(maker_account.clone()),
			1,
			trading_pair
		));
		assert_ok!(OCEX::claim_lmp_rewards(
			RuntimeOrigin::signed(taker_account.clone()),
			1,
			trading_pair
		));
	})
}

#[test]
#[sequential]
fn test_on_chain_validation_with_auction() {
	new_test_ext().execute_with(|| {
		let recipient_address = AccountId32::new([2; 32]);
		let pot_account: AccountId32 = OCEX::get_pot_account();
		let pallet_account: AccountId32 = OCEX::get_pallet_account();
		Balances::mint_into(&pot_account, 10 * UNIT_BALANCE);
		Balances::mint_into(&pallet_account, 20 * UNIT_BALANCE);
		let auction_duration = 100;
		let burn_ration = 50;
		let fee_distribution = FeeDistribution {
			recipient_address: recipient_address.clone(),
			auction_duration,
			burn_ration,
		};
		assert_ok!(OCEX::set_fee_distribution(RuntimeOrigin::root(), fee_distribution));
		set_lmp_config();
		Assets::mint_into(1u128, &pallet_account, 1000 * UNIT_BALANCE).unwrap();
		push_trade_user_actions(1, 1, 1);
		assert_eq!(Balances::free_balance(&recipient_address), 0);
		assert_ok!(OCEX::run_on_chain_validation(1));
		let snapshot_id: u64 = 1;
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());
		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref
			.get::<(SnapshotSummary<AccountId32>, crate::sr25519::AuthoritySignature, u16)>()
		{
			Ok(Some((summary, signature, index))) => {
				println!("Summary {:?}", summary);
				assert_eq!(summary.snapshot_id, 1);
				assert_eq!(summary.state_change_id, 1);
				assert_eq!(summary.last_processed_blk, 1);
				assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), summary, Vec::new()));
			},
			_ => panic!("Snapshot not found"),
		};
		OCEX::start_new_epoch(2);
		push_trade_user_actions_with_fee(2, 1, 2);
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		s_info.set(&false);
		assert_ok!(OCEX::run_on_chain_validation(2));
		let snapshot_id: u64 = 2;
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());
		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref
			.get::<(SnapshotSummary<AccountId32>, crate::sr25519::AuthoritySignature, u16)>()
		{
			Ok(Some((summary, signature, index))) => {
				println!("Summary {:?}", summary);
				assert_eq!(summary.snapshot_id, 2);
				assert_eq!(summary.state_change_id, 2);
				assert_eq!(summary.last_processed_blk, 3);
				assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), summary, Vec::new()));
			},
			_ => panic!("Snapshot not found"),
		};
		assert_eq!(Balances::free_balance(&recipient_address), 10000000000);
	})
}

pub fn set_lmp_config() {
	let total_liquidity_mining_rewards: Option<Compact<u128>> =
		Some(Compact::from(1000 * UNIT_BALANCE));
	let total_trading_rewards: Option<Compact<u128>> = Some(Compact::from(1000 * UNIT_BALANCE));
	let reward_pallet_account = OCEX::get_pallet_account();
	assert_ok!(Balances::mint_into(&reward_pallet_account, 1100 * UNIT_BALANCE));
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
	// Register trading pair
	Balances::mint_into(&AccountId32::new([1; 32]), UNIT_BALANCE).unwrap();
	assert_ok!(Assets::create(
		RuntimeOrigin::signed(AccountId32::new([1; 32])),
		parity_scale_codec::Compact(quote_asset.asset_id().unwrap()),
		AccountId32::new([1; 32]),
		One::one()
	));
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), base_asset));
	assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), quote_asset));
	assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
	assert_ok!(OCEX::register_trading_pair(
		RuntimeOrigin::root(),
		base_asset,
		quote_asset,
		(1_0000_0000_u128 * 1_000_000_u128).into(),
		(1_000_000_000_000_000_u128 * 1_000_u128).into(),
		1_000_000_u128.into(),
		1_0000_0000_u128.into(),
	));
	let max_accounts_rewarded: Option<u16> = Some(10);
	let claim_safety_period: Option<u32> = Some(0);
	let lmp_config = LMPMarketConfigWrapper {
		trading_pair,
		market_weightage: UNIT_BALANCE,
		min_fees_paid: UNIT_BALANCE,
		min_maker_volume: UNIT_BALANCE,
		max_spread: UNIT_BALANCE,
		min_depth: UNIT_BALANCE,
	};
	assert_ok!(OCEX::set_lmp_epoch_config(
		RuntimeOrigin::root(),
		total_liquidity_mining_rewards,
		total_trading_rewards,
		vec![lmp_config],
		max_accounts_rewarded,
		claim_safety_period
	));
	OCEX::start_new_epoch(1);
}

fn push_trade_user_actions_with_fee(stid: u64, snapshot_id: u64, block_no: u64) {
	let (maker_trade, taker_trade) = get_trades();

	let trade = Trade {
		maker: maker_trade,
		taker: taker_trade,
		price: Decimal::from_f64(0.8).unwrap(),
		amount: Decimal::from(10),
		time: 0,
	};
	let block_no = get_block_import(block_no);
	let ingress_message = IngressMessages::WithdrawTradingFees;
	let mut fees_map: BTreeMap<AssetId, Decimal> = BTreeMap::new();
	fees_map.insert(AssetId::Polkadex, Decimal::from_f64(0.020).unwrap());
	fees_map.insert(AssetId::Asset(1), Decimal::from_f64(0.0160).unwrap());
	let egress_message = EgressMessages::TradingFees(fees_map);
	let mut ie_map = BTreeMap::new();
	ie_map.insert(ingress_message.clone(), egress_message);
	<crate::pallet::IngressMessages<Test>>::insert(
		block_no.saturating_add(1),
		vec![ingress_message],
	);
	let block_import_action =
		UserActions::BlockImport(block_no as u32, BTreeMap::new(), BTreeMap::new());
	let block_import_with_tp =
		UserActions::BlockImport(block_no.saturating_add(1) as u32, ie_map, BTreeMap::new());
	let trade_action = UserActions::Trade(vec![trade]);
	let user_action_batch = UserActionBatch {
		actions: vec![block_import_action, trade_action, block_import_with_tp],
		stid,
		snapshot_id,
		signature: sp_core::ecdsa::Signature::from_raw([0; 65]),
	};
	AggregatorClient::<Test>::mock_get_user_action_batch(user_action_batch);
}

fn push_trade_user_actions(stid: u64, snapshot_id: u64, block_no: u64) {
	let (maker_trade, taker_trade) = get_trades();

	let trade = Trade {
		maker: maker_trade,
		taker: taker_trade,
		price: Decimal::from_f64(0.8).unwrap(),
		amount: Decimal::from(10),
		time: 0,
	};
	let block_no = get_block_import(block_no);
	let block_import_action =
		UserActions::BlockImport(block_no as u32, BTreeMap::new(), BTreeMap::new());
	let trade_action = UserActions::Trade(vec![trade]);
	let user_action_batch = UserActionBatch {
		actions: vec![block_import_action, trade_action],
		stid,
		snapshot_id,
		signature: sp_core::ecdsa::Signature::from_raw([0; 65]),
	};
	AggregatorClient::<Test>::mock_get_user_action_batch(user_action_batch);
}

fn get_block_import(block_no: u64) -> u64 {
	let block_no = block_no;
	let (maker_account, taker_account) = get_maker_and_taker__account();
	let maker_ingress_message =
		IngressMessages::Deposit(maker_account, AssetId::Asset(1), Decimal::from(100));
	let taker_ingress_message =
		IngressMessages::Deposit(taker_account, AssetId::Polkadex, Decimal::from(100));
	<IngressMessagesStorage<Test>>::insert(
		block_no,
		vec![maker_ingress_message, taker_ingress_message],
	);
	block_no
}

fn get_maker_and_taker__account() -> (AccountId32, AccountId32) {
	let (maker_user_pair, _) = sp_core::sr25519::Pair::from_phrase(
		"spider sell nice animal border success square soda stem charge caution echo",
		None,
	)
	.unwrap();
	let (taker_user_pair, _) = sp_core::sr25519::Pair::from_phrase(
		"ketchup route purchase humble harsh true glide chef buyer crane infant sponsor",
		None,
	)
	.unwrap();
	(AccountId32::from(maker_user_pair.public().0), AccountId32::from(taker_user_pair.public().0))
}

fn get_trades() -> (Order, Order) {
	let (maker_user_pair, _) = sp_core::sr25519::Pair::from_phrase(
		"spider sell nice animal border success square soda stem charge caution echo",
		None,
	)
	.unwrap();
	<Accounts<Test>>::insert(
		AccountId32::new((maker_user_pair.public().0)),
		AccountInfo::new(AccountId32::new((maker_user_pair.public().0))),
	);
	let trading_pair = TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) }; //PDEX(Base)/USDT(Quote)
	let trading_pair_config = TradingPairConfig {
		base_asset: trading_pair.base.clone(),
		quote_asset: trading_pair.quote.clone(),
		price_tick_size: Decimal::from_f64(0.1).unwrap(),
		min_volume: Decimal::from(1),
		max_volume: Decimal::from(100),
		qty_step_size: Decimal::from_f64(0.1).unwrap(),
		operational_status: true,
		base_asset_precision: 12,
		quote_asset_precision: 12,
	};
	<TradingPairs<Test>>::insert(
		trading_pair.base.clone(),
		trading_pair.quote.clone(),
		trading_pair_config,
	);
	let mut maker_order = Order {
		//User is buying PDEX - User has USDT
		stid: 0,
		client_order_id: H256::from_low_u64_be(1),
		avg_filled_price: Decimal::from(2),
		fee: Decimal::from(1),
		filled_quantity: Decimal::from(1),
		status: OrderStatus::OPEN,
		id: H256::from_low_u64_be(1),
		user: AccountId32::new((maker_user_pair.public().0)),
		main_account: AccountId32::new((maker_user_pair.public().0)),
		pair: trading_pair,
		side: OrderSide::Bid,
		order_type: OrderType::LIMIT,
		qty: Decimal::from(10),              //How much PDEX user wants to buy
		price: Decimal::from(1),             //For how much USDT (1 PDEX) - user wants to buy PDEX
		quote_order_qty: Default::default(), //Check with @gautham
		timestamp: 0,
		overall_unreserved_volume: Default::default(), //Check with @gautham
		signature: Signature::from_raw([1; 64]).into(),
	};
	let order_payload: OrderPayload = maker_order.clone().into();
	// Sign order_payload
	let signature = maker_user_pair.sign(&order_payload.encode());
	maker_order.signature = signature.into();

	let (taker_user_pair, _) = sp_core::sr25519::Pair::from_phrase(
		"ketchup route purchase humble harsh true glide chef buyer crane infant sponsor",
		None,
	)
	.unwrap();
	<Accounts<Test>>::insert(
		AccountId32::new((taker_user_pair.public().0)),
		AccountInfo::new(AccountId32::new((taker_user_pair.public().0))),
	);
	let mut taker_order = Order {
		//User is selling PDEX - User has PDEX
		stid: 0,
		client_order_id: H256::from_low_u64_be(2),
		avg_filled_price: Decimal::from(2),
		fee: Decimal::from(1),
		filled_quantity: Decimal::from(1),
		status: OrderStatus::OPEN,
		id: H256::from_low_u64_be(1),
		user: AccountId32::new((taker_user_pair.public().0)),
		main_account: AccountId32::new((taker_user_pair.public().0)),
		pair: trading_pair,
		side: OrderSide::Ask,
		order_type: OrderType::LIMIT,
		qty: Decimal::from(15),                 //How much PDEX user wants to sell
		price: Decimal::from_f64(0.8).unwrap(), //For how much USDT (1 PDEX) - user wants to sell PDEX
		quote_order_qty: Default::default(),    //Check with @gautham
		timestamp: 0,
		overall_unreserved_volume: Default::default(), //Check with @gautham
		signature: Signature::from_raw([1; 64]).into(),
	};
	let order_payload: OrderPayload = taker_order.clone().into();
	// Sign order_payload
	let signature = taker_user_pair.sign(&order_payload.encode());
	taker_order.signature = signature.into();
	(maker_order, taker_order)
}
