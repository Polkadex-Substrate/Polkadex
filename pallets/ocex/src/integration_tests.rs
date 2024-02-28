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

use frame_support::assert_ok;
use sp_core::crypto::AccountId32;
use orderbook_primitives::types::{Order, OrderSide, OrderStatus, OrderType, Trade, TradingPair, UserActionBatch};
use orderbook_primitives::types::UserActions;
use polkadex_primitives::AssetId;
use crate::mock::*;
use crate::mock::new_test_ext;


#[test]
fn test_run_on_chain_validation_happy_path() {
    new_test_ext().execute_with(|| {

        assert_ok!(OCEX::run_on_chain_validation(1));
    })
}


fn push_user_actions() {
    let actions = vec![];
    let trade_action = UserActions::Trade(Trade {
        maker: Order {
            stid: 0,
            client_order_id: Default::default(),
            avg_filled_price: Default::default(),
            fee: Default::default(),
            filled_quantity: Default::default(),
            status: OrderStatus::OPEN,
            id: Default::default(),
            user: (),
            main_account: (),
            pair: TradingPair { base: AssetId::Polkadex, quote: AssetId::Polkadex },
            side: OrderSide::Ask,
            order_type: OrderType::LIMIT,
            qty: Default::default(),
            price: Default::default(),
            quote_order_qty: Default::default(),
            timestamp: 0,
            overall_unreserved_volume: Default::default(),
            signature: (),
        },
        taker: Order {},
        price: Default::default(),
        amount: Default::default(),
        time: 0,
    });
    let user_action_batch = UserActionBatch {
        actions: vec![],
        stid: 0,
        snapshot_id: 0,
        signature: Default::default(),
    };
}
