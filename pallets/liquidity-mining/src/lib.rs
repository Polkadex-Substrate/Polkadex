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

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use orderbook_primitives::LiquidityMining;
	use orderbook_primitives::types::TradingPair;


	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

		/// Some type that implements the LiquidityMining traits
		type OCEX: LiquidityMining;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Market is not registered with OCEX pallet
		UnknownMarket
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new pool
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn register_pool(origin: OriginFor<T>, name: [u8;10], market: TradingPair, commission: u128, exit_fee: u128) -> DispatchResult {
			// Check market is active
			ensure!(T::OCEX::is_registered_market(&market), Error::<T>::UnknownMarket);
			// Check if commission is between 0-1
			let mut commission = Decimal::
			// Check if exit_fee is between 0 -1
			// Create the a pool address with origin and market combo if it doesn't exist
			// Register on OCEX pallet
			// Start cycle
			Ok(())
		}
	}
}
