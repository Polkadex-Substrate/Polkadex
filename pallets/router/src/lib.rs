// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Router for Automatic Market Maker (AMM)
//!
//! Given a supported `route`, executes the indicated trades on all the available AMM(s) pool(s).

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure, log,
		pallet_prelude::{DispatchResult, DispatchResultWithPostInfo, Weight},
		require_transactional,
		traits::{
			fungibles::{Inspect, Mutate},
			tokens::{Fortitude, Preservation},
			Get, IsType,
		},
		transactional, BoundedVec, PalletId,
	};
	use frame_system::{ensure_signed, pallet_prelude::OriginFor};
	use polkadex_primitives::Balance;
	use sp_runtime::{traits::Zero, DispatchError};
	use sp_std::{cmp::Reverse, collections::btree_map::BTreeMap, vec::Vec};
	use support::AMM;

	pub type Route<T, I> = BoundedVec<
		(
			// Base asset
			AssetIdOf<T, I>,
			// Quote asset
			AssetIdOf<T, I>,
		),
		<T as Config<I>>::MaxLengthRoute,
	>;

	pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub(crate) type AssetIdOf<T, I = ()> =
		<<T as Config<I>>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
	pub(crate) type BalanceOf<T, I = ()> =
		<<T as Config<I>>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Router pallet id
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Specify all the AMMs we are routing between
		type AMM: AMM<AccountIdOf<Self>, AssetIdOf<Self, I>, BalanceOf<Self, I>, Self::BlockNumber>;

		/// Weight information for extrinsics in this pallet.
		// type AMMRouterWeightInfo: WeightInfo;

		/// How many routes we support at most
		#[pallet::constant]
		type MaxLengthRoute: Get<u32>;

		/// The asset id for native currency.
		#[pallet::constant]
		type GetNativeCurrencyId: Get<AssetIdOf<Self, I>>;

		/// Currency type for deposit/withdraw assets to/from amm route
		/// module
		type Assets: Inspect<Self::AccountId, AssetId = u128, Balance = Balance>
			+ Mutate<Self::AccountId, AssetId = u128, Balance = Balance>;
	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Input balance must not be zero
		ZeroBalance,
		/// Must input one route at least
		EmptyRoute,
		/// User hasn't enough tokens for transaction
		InsufficientBalance,
		/// Exceed the max length of routes we allow
		ExceedMaxLengthRoute,
		/// Input duplicated route
		DuplicatedRoute,
		/// A more specific UnexpectedSlippage when trading exact amount out
		MaximumAmountInViolated,
		/// A more specific UnexpectedSlippage when trading exact amount in
		MinimumAmountOutViolated,
		/// Token doesn't exists in all pools
		TokenDoesNotExists,
		/// Route between tokens is not possible
		NoPossibleRoute,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Event emitted when swap is successful
		/// [sender, amount_in, route, amount_out]
		Traded(T::AccountId, BalanceOf<T, I>, Vec<AssetIdOf<T, I>>, BalanceOf<T, I>),
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Check that routes are unique and that the length > 0 and < MaxLengthRoute
		#[require_transactional]
		pub fn route_checks(route: &[AssetIdOf<T, I>]) -> DispatchResult {
			// Ensure the length of routes should be >= 1 at least.
			ensure!(!route.is_empty(), Error::<T, I>::EmptyRoute);

			// Ensure user do not input too many routes.
			ensure!(
				route.len() <= T::MaxLengthRoute::get() as usize,
				Error::<T, I>::ExceedMaxLengthRoute
			);

			// check for duplicates with O(n^2) complexity
			// only good for short routes and we have a cap checked above
			let contains_duplicate = (1..route.len()).any(|i| route[i..].contains(&route[i - 1]));

			// Ensure user doesn't input duplicated routes (a cycle in the graph)
			ensure!(!contains_duplicate, Error::<T, I>::DuplicatedRoute);

			Ok(())
		}

		/// Returns a sorted list of all routes and their output amounts from a
		/// start token to end token by traversing a graph.
		#[allow(clippy::all)]
		pub fn get_all_routes(
			amount: BalanceOf<T, I>,
			token_in: AssetIdOf<T, I>,
			token_out: AssetIdOf<T, I>,
			reversed: bool,
		) -> Result<Vec<(Vec<AssetIdOf<T, I>>, BalanceOf<T, I>)>, DispatchError> {
			// get all the pool asset pairs from the AMM
			let pools = T::AMM::get_pools()?;

			let mut graph: BTreeMap<u128, Vec<u128>> = BTreeMap::new();

			// build a non directed graph from pool asset pairs
			pools.into_iter().for_each(|(a, b)| {
				graph.entry(a).or_insert_with(Vec::new).push(b);
				graph.entry(b).or_insert_with(Vec::new).push(a);
			});

			// init mutable variables
			let mut path = Vec::new();
			let mut paths = Vec::new();

			let mut start = token_in;
			let mut end = token_out;

			let mut queue: Vec<(u128, u128, Vec<u128>)> = Vec::from([(start, end, path)]);

			// check that both tokens exist in graph
			ensure!(graph.contains_key(&start), Error::<T, I>::TokenDoesNotExists);
			ensure!(graph.contains_key(&end), Error::<T, I>::TokenDoesNotExists);

			// iterate until we build all routes
			while !queue.is_empty() {
				// desugared RFC 2909-destructuring-assignment
				let (_start, _end, _path) = queue.swap_remove(0);
				start = _start;
				end = _end;
				path = _path;

				path.push(start);

				// exit if we reached our target
				if start == end {
					paths.push(path.clone());
				}

				// cant error because we fetch pools above
				let adjacents = graph.get(&start).unwrap();

				// items that are adjacent but not already in path
				let difference: Vec<_> =
					adjacents.iter().filter(|item| !path.contains(item)).collect();

				for node in difference {
					queue.push((*node, end, path.clone()));
				}
			}

			// get output amounts for all routes
			let mut output_routes = Self::get_output_routes(amount, paths, reversed);

			// sort values greatest to least
			output_routes.sort_by_key(|k| Reverse(k.1));

			Ok(output_routes)
		}

		/// Returns the route that results in the largest amount out for amount in
		#[allow(clippy::all)]
		pub fn get_best_route(
			amount: BalanceOf<T, I>,
			token_in: AssetIdOf<T, I>,
			token_out: AssetIdOf<T, I>,
			reversed: bool,
		) -> Result<(Vec<AssetIdOf<T, I>>, BalanceOf<T, I>), DispatchError> {
			let mut all_routes = Self::get_all_routes(amount, token_in, token_out, reversed)?;
			ensure!(!all_routes.is_empty(), Error::<T, I>::NoPossibleRoute);
			let best_route = if reversed {
				all_routes.remove(all_routes.len() - 1)
			} else {
				all_routes.remove(0)
			};

			log::trace!(
				target: "router::get_best_route",
				"amount: {:?}, token_in: {:?}, token_out: {:?}, reversed: {:?}, best_route: {:?}",
				amount,
				token_in,
				token_out,
				reversed,
				best_route
			);

			Ok(best_route)
		}

		///  Returns output routes for given amount from all available routes
		#[allow(clippy::all)]
		pub fn get_output_routes(
			amount: BalanceOf<T, I>,
			routes: Vec<Vec<AssetIdOf<T, I>>>,
			reversed: bool,
		) -> Vec<(Vec<AssetIdOf<T, I>>, BalanceOf<T, I>)> {
			let mut output_routes = Vec::new();

			if reversed {
				for route in routes {
					let amounts = T::AMM::get_amounts_in(amount, route.clone());
					if let Ok(amounts) = amounts {
						output_routes.push((route, amounts[0]));
					}
				}
			} else {
				for route in routes {
					let amounts = T::AMM::get_amounts_out(amount, route.clone());
					if let Ok(amounts) = amounts {
						output_routes.push((route, amounts[amounts.len() - 1]));
					}
				}
			}

			output_routes
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Given input amount is fixed, the output token amount is not known in advance.
		///
		/// # Parameters
		///
		/// * `origin`: the trader.
		/// * `route`: the route user inputs.
		/// * `amount_in`: the amount of trading assets.
		/// * `min_amount_out`: the minimum a trader is willing to receive.
		#[transactional]
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn swap_exact_tokens_for_tokens(
			origin: OriginFor<T>,
			route: Vec<AssetIdOf<T, I>>,
			#[pallet::compact] amount_in: BalanceOf<T, I>,
			#[pallet::compact] min_amount_out: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let trader = ensure_signed(origin)?;

			// do all checks on routes
			Self::route_checks(&route)?;

			// Ensure balances user input is bigger than zero.
			ensure!(
				amount_in > Zero::zero() && min_amount_out >= Zero::zero(),
				Error::<T, I>::ZeroBalance
			);

			// Ensure the trader has enough tokens for transaction.
			let from_currency_id = route[0];
			ensure!(
				T::Assets::reducible_balance(
					from_currency_id,
					&trader,
					Preservation::Preserve,
					Fortitude::Polite
				) >= amount_in,
				Error::<T, I>::InsufficientBalance
			);

			let amounts = T::AMM::get_amounts_out(amount_in, route.clone())?;

			// make sure the required amount in does not violate our input
			ensure!(
				amounts[amounts.len() - 1] >= min_amount_out,
				Error::<T, I>::MinimumAmountOutViolated
			);

			for i in 0..(route.len() - 1) {
				let next_index = i + 1;
				T::AMM::swap(&trader, (route[i], route[next_index]), amounts[i])?;
			}

			Self::deposit_event(Event::Traded(
				trader,
				amounts[0],
				route,
				amounts[amounts.len() - 1],
			));

			Ok(().into())
		}

		/// Given the output token amount is fixed, the input token amount is not known.
		///
		/// * `origin`: the trader.
		/// * `route`: the route user inputs.
		/// * `amount_out`: the amount of trading assets.
		/// * `max_amount_in`: the maximum a trader is willing to input.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		#[transactional]
		pub fn swap_tokens_for_exact_tokens(
			origin: OriginFor<T>,
			route: Vec<AssetIdOf<T, I>>,
			#[pallet::compact] amount_out: BalanceOf<T, I>,
			#[pallet::compact] max_amount_in: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			let trader = ensure_signed(origin)?;

			// do all checks on routes
			Self::route_checks(&route)?;

			// Ensure balances user input is bigger than zero.
			ensure!(
				amount_out > Zero::zero() && max_amount_in >= Zero::zero(),
				Error::<T, I>::ZeroBalance
			);

			// calculate trading amounts
			let amounts = T::AMM::get_amounts_in(amount_out, route.clone())?;

			// we need to check after calc so we know how much is expected to be input
			// Ensure the trader has enough tokens for transaction.
			let from_currency_id = route[0];
			ensure!(
				T::Assets::reducible_balance(
					from_currency_id,
					&trader,
					Preservation::Preserve,
					Fortitude::Polite
				) > amounts[0],
				Error::<T, I>::InsufficientBalance
			);

			// make sure the required amount in does not violate our input
			ensure!(max_amount_in >= amounts[0], Error::<T, I>::MaximumAmountInViolated);

			for i in 0..(route.len() - 1) {
				let next_index = i + 1;
				T::AMM::swap(&trader, (route[i], route[next_index]), amounts[i])?;
			}

			Self::deposit_event(Event::Traded(
				trader,
				amounts[0],
				route,
				amounts[amounts.len() - 1],
			));

			Ok(().into())
		}
	}
}
