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

use std::collections::BTreeMap;
use frame_support::assert_ok;
use num_traits::FromPrimitive;
use parity_scale_codec::Encode;
use rust_decimal::Decimal;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::AccountId32;
use sp_core::{H256, Pair};
use sp_core::sr25519::Signature;
use sp_runtime::offchain::storage::StorageValueRef;
use orderbook_primitives::SnapshotSummary;
use orderbook_primitives::types::{Order, OrderPayload, OrderSide, OrderStatus, OrderType, Trade, TradingPair, UserActionBatch};
use orderbook_primitives::types::UserActions;
use polkadex_primitives::AssetId;
use polkadex_primitives::ocex::{AccountInfo, TradingPairConfig};
use crate::aggregator::AggregatorClient;
use crate::mock::*;
use crate::mock::new_test_ext;
use crate::pallet::{Accounts, TradingPairs, IngressMessages as IngressMessagesStorage};
use polkadex_primitives::ingress::IngressMessages;
use crate::Config;
use crate::snapshot::StateInfo;
use crate::storage::{OffchainState, store_trie_root};
use crate::validator::LAST_PROCESSED_SNAPSHOT;


#[test]
fn test_run_on_chain_validation_trades_happy_path() {
    new_test_ext().execute_with(|| {
        push_trade_user_actions();
        assert_ok!(OCEX::run_on_chain_validation(1));
        let snapshot_id:u64 = 1;
        let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
        key.append(&mut snapshot_id.encode());
        let summay_ref = StorageValueRef::persistent(&key);
        match summay_ref.get::<(
            SnapshotSummary<AccountId32>,
            crate::sr25519::AuthoritySignature,
            u16,
        )>() {
            Ok(Some((summary, signature, index))) => {
                assert_eq!(summary.snapshot_id, 1);
                assert_eq!(summary.state_change_id, 1);
                assert_eq!(summary.last_processed_blk, 4768084);
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
             }};
        assert_eq!(state_info.last_block, 4768084);
        assert_eq!(state_info.stid, 1);
        assert_eq!(state_info.snapshot_id, 1);
    });
}


fn push_trade_user_actions() {
    let (maker_trade, taker_trade) = get_trades();

    let trade = Trade {
        maker: maker_trade,
        taker: taker_trade,
        price: Decimal::from_f64(0.8).unwrap(),
        amount: Decimal::from(10),
        time: 0,
    };
    let block_no = get_block_import();
    let block_import_action = UserActions::BlockImport(block_no as u32, BTreeMap::new(), BTreeMap::new());
    let trade_action = UserActions::Trade(vec![trade]);
    let user_action_batch = UserActionBatch {
        actions: vec![block_import_action, trade_action],
        stid: 1,
        snapshot_id: 0,
        signature: sp_core::ecdsa::Signature::from_raw([0;65]),
    };
    AggregatorClient::<Test>::mock_get_user_action_batch(user_action_batch);
}

fn get_block_import() -> u64 {
    let block_no = 4768084;
    let (maker_account, taker_account) = get_maker_and_taker__account();
    let maker_ingress_message = IngressMessages::Deposit(maker_account, AssetId::Asset(1), Decimal::from(100));
    let taker_ingress_message = IngressMessages::Deposit(taker_account, AssetId::Polkadex, Decimal::from(100));
    <IngressMessagesStorage<Test>>::insert(block_no, vec![maker_ingress_message, taker_ingress_message]);
    block_no
}

fn get_maker_and_taker__account() -> (AccountId32, AccountId32) {
    let (maker_user_pair, _) = sp_core::sr25519::Pair::from_phrase("spider sell nice animal border success square soda stem charge caution echo", None).unwrap();
    let (taker_user_pair, _) = sp_core::sr25519::Pair::from_phrase("ketchup route purchase humble harsh true glide chef buyer crane infant sponsor", None).unwrap();
    (AccountId32::from(maker_user_pair.public().0), AccountId32::from(taker_user_pair.public().0))
}

// fn update_offchain_storage_state() {
//     let mut root = crate::storage::load_trie_root();
//     let mut storage = crate::storage::State;
//     let mut state = OffchainState::load(&mut storage, &mut root);
//     let state_info = StateInfo {
//         last_block: 0,
//         worker_nonce: 0,
//         stid: 1,
//         snapshot_id: 0,
//     };
//     OCEX::store_state_info(state_info, &mut state);
// }

fn get_trades() -> (Order, Order) {
    let (maker_user_pair, _) = sp_core::sr25519::Pair::from_phrase("spider sell nice animal border success square soda stem charge caution echo", None).unwrap();
    <Accounts<Test>>::insert(AccountId32::new((maker_user_pair.public().0)), AccountInfo::new(AccountId32::new((maker_user_pair.public().0))));
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
    <TradingPairs<Test>>::insert(trading_pair.base.clone(), trading_pair.quote.clone(), trading_pair_config);
    let mut maker_order = Order { //User is buying PDEX - User has USDT
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
        qty: Decimal::from(10), //How much PDEX user wants to buy
        price: Decimal::from(1), //For how much USDT (1 PDEX) - user wants to buy PDEX
        quote_order_qty: Default::default(), //Check with @gautham
        timestamp: 0,
        overall_unreserved_volume: Default::default(), //Check with @gautham
        signature: Signature::from_raw([1;64]).into(),
    };
    let order_payload: OrderPayload = maker_order.clone().into();
    // Sign order_payload
    let signature = maker_user_pair.sign(&order_payload.encode());
    maker_order.signature = signature.into();

    let (taker_user_pair, _) = sp_core::sr25519::Pair::from_phrase("ketchup route purchase humble harsh true glide chef buyer crane infant sponsor", None).unwrap();
    <Accounts<Test>>::insert(AccountId32::new((taker_user_pair.public().0)), AccountInfo::new(AccountId32::new((taker_user_pair.public().0))));
    let mut taker_order = Order { //User is selling PDEX - User has PDEX
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
        qty: Decimal::from(15), //How much PDEX user wants to sell
        price: Decimal::from_f64(0.8).unwrap(), //For how much USDT (1 PDEX) - user wants to sell PDEX
        quote_order_qty: Default::default(), //Check with @gautham
        timestamp: 0,
        overall_unreserved_volume: Default::default(), //Check with @gautham
        signature: Signature::from_raw([1;64]).into(),
    };
    let order_payload: OrderPayload = taker_order.clone().into();
    // Sign order_payload
    let signature = taker_user_pair.sign(&order_payload.encode());
    taker_order.signature = signature.into();
    (maker_order, taker_order)
}
