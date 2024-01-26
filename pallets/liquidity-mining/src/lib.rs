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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

use sp_std::vec::Vec;

mod callback;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use crate::types::MarketMakerConfig;
	use core::ops::{Div, DivAssign, MulAssign};
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{traits::AccountIdConversion, SaturatedConversion},
		traits::{
			fungibles::{Create, Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
		transactional, PalletId,
	};
	use frame_system::{
		offchain::{SendTransactionTypes, SubmitTransaction},
		pallet_prelude::*,
	};
	use orderbook_primitives::{constants::UNIT_BALANCE, types::TradingPair, LiquidityMining};
	use polkadex_primitives::AssetId;
	use rust_decimal::{prelude::*, Decimal};
	use sp_io::hashing::blake2_128;
	use sp_runtime::{
		traits::{CheckedDiv, UniqueSaturatedInto},
		Saturating,
	};
	use sp_std::collections::btree_map::BTreeMap;

	type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	type SumOfScores<T> = BalanceOf<T>;
	type MMScore<T> = BalanceOf<T>;
	type MMClaimFlag = bool;
	type MMInfo<T> = (
		BTreeMap<<T as frame_system::Config>::AccountId, (MMScore<T>, MMClaimFlag)>,
		SumOfScores<T>,
		MMClaimFlag,
	);

	type LMPScoreSheet<T> = BTreeMap<
		(TradingPair, <T as frame_system::Config>::AccountId, u16),
		(BTreeMap<<T as frame_system::Config>::AccountId, (BalanceOf<T>, bool)>, BalanceOf<T>),
	>;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

		/// Some type that implements the LiquidityMining traits
		type OCEX: LiquidityMining<Self::AccountId, BalanceOf<Self>>;

		/// Pool Accounts are derived from this
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Balances Pallet
		type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Assets Pallet
		type OtherAssets: Mutate<
				<Self as frame_system::Config>::AccountId,
				Balance = BalanceOf<Self>,
				AssetId = u128,
			> + Inspect<<Self as frame_system::Config>::AccountId>
			+ Create<<Self as frame_system::Config>::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// LP Shares
	#[pallet::storage]
	pub(super) type LPShares<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u128, // share_id
		Identity,
		T::AccountId, // LP
		BalanceOf<T>,
		ValueQuery,
	>;

	/// Pools
	#[pallet::storage]
	#[pallet::getter(fn lmp_pool)]
	pub(super) type Pools<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TradingPair, // market
		Identity,
		T::AccountId, // market maker
		MarketMakerConfig<T::AccountId>,
		OptionQuery,
	>;

	/// Rewards by Pool
	#[pallet::storage]
	#[pallet::getter(fn rewards_by_pool)]
	pub(super) type Rewards<T: Config> = StorageDoubleMap<
		_,
		Identity,
		u16, // market
		Identity,
		T::AccountId, // pool_id
		BalanceOf<T>,
		OptionQuery,
	>;

	/// Record of multiple LP deposits per epoch
	#[pallet::storage]
	pub(super) type AddLiquidityRecords<T: Config> = StorageDoubleMap<
		_,
		Identity,
		u16, // epoch
		Identity,
		(T::AccountId, T::AccountId),           // (pool_id,lp)
		Vec<(BlockNumberFor<T>, BalanceOf<T>)>, // List of deposits and their blk number per epoch
		ValueQuery,
	>;

	/// Withdrawal Requests
	#[pallet::storage]
	pub(super) type WithdrawalRequests<T: Config> = StorageDoubleMap<
		_,
		Identity,
		u16, // epoch
		Identity,
		T::AccountId,                                    // pool_id
		Vec<(T::AccountId, BalanceOf<T>, BalanceOf<T>)>, // List of pending requests
		ValueQuery,
	>;

	/// Liquidity Providers map
	#[pallet::storage]
	#[pallet::getter(fn liquidity_providers)]
	pub(super) type LiquidityProviders<T: Config> = StorageDoubleMap<
		_,
		Identity,
		u16, // Epoch
		Identity,
		T::AccountId, // Pool address
		MMInfo<T>,
		ValueQuery,
	>;

	/// Active LMP Epoch
	#[pallet::storage]
	#[pallet::getter(fn active_lmp_epoch)]
	pub(crate) type LMPEpoch<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Offchain worker flag
	#[pallet::storage]
	pub(super) type SnapshotFlag<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Issueing withdrawals for epoch
	#[pallet::storage]
	pub(super) type WithdrawingEpoch<T: Config> = StorageValue<_, u16, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		LiquidityAdded {
			market: TradingPair,
			pool: T::AccountId,
			lp: T::AccountId,
			shares: BalanceOf<T>,
			share_id: AssetId,
			price: BalanceOf<T>,
			total_inventory_in_quote: BalanceOf<T>,
		},
		LiquidityRemoved {
			market: TradingPair,
			pool: T::AccountId,
			lp: T::AccountId,
		},
		LiquidityRemovalFailed {
			market: TradingPair,
			pool: T::AccountId,
			lp: T::AccountId,
			burn_frac: BalanceOf<T>,
			base_free: BalanceOf<T>,
			quote_free: BalanceOf<T>,
			base_required: BalanceOf<T>,
			quote_required: BalanceOf<T>,
		},
		PoolForceClosed {
			market: TradingPair,
			pool: T::AccountId,
			base_freed: BalanceOf<T>,
			quote_freed: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Market is not registered with OCEX pallet
		UnknownMarket,
		/// Decimal Conversion error
		ConversionError,
		/// Commission should be between 0-1
		InvalidCommission,
		/// Exit fee should be between 0-1
		InvalidExitFee,
		/// Pool already exists
		PoolExists,
		/// There is not enough quote for given base amount
		NotEnoughQuoteAmount,
		/// Pool is not registered
		UnknownPool,
		/// Public deposits not allowed in this pool
		PublicDepositsNotAllowed,
		/// Total share issuance is zero(this should never happen)
		TotalShareIssuanceIsZero,
		/// LP not found in map
		InvalidLPAddress,
		/// Reward already claimed
		AlreadyClaimed,
		/// Invalid Total Score
		InvalidTotalScore,
		/// Pool is force closed, add liquidity not allowed
		PoolForceClosed,
		/// Pool is not force closed to claim funds
		PoolNotForceClosed,
		/// Invalid Total issuance number
		InvalidTotalIssuance,
		/// Snapshotting in progress, try again later
		SnapshotInProgress,
		/// Price Oracle not available, try again later
		PriceNotAvailable,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Weight::zero()
		}

		fn offchain_worker(_: BlockNumberFor<T>) {
			Self::take_snapshot();
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_scores_of_lps { results: _ } = call {
				// This txn is only available during snapshotting
				if <SnapshotFlag<T>>::get().is_none() {
					return InvalidTransaction::Call.into()
				}
				if source == TransactionSource::External {
					// Don't accept externally sourced calls
					return InvalidTransaction::Call.into()
				}

				// TODO: @zktony Update the verification logic to make it more stringent.
				ValidTransaction::with_tag_prefix("LiquidityMining")
					// We set base priority to 2**20 and hope it's included before any other
					// transactions in the pool. Next we tweak the priority depending on how much
					// it differs from the current average. (the more it differs the more priority
					// it has).
					.priority(Default::default()) // TODO: update this
					// This transaction does not require anything else to go before into the pool.
					// In theory we could require `previous_unsigned_at` transaction to go first,
					// but it's not necessary in our case.
					//.and_requires()
					// We set the `provides` tag to be the same as `next_unsigned_at`. This makes
					// sure only one transaction produced after `next_unsigned_at` will ever
					// get to the transaction pool and will end up in the block.
					// We can still have multiple transactions compete for the same "spot",
					// and the one with higher priority will replace other one in the pool.
					.and_provides("liquidity_mining") // TODO: update this
					// The transaction is only valid for next 5 blocks. After that it's
					// going to be revalidated by the pool.
					.longevity(5)
					// It's fine to propagate that transaction to other peers, which means it can be
					// created even by nodes that don't produce blocks.
					// Note that sometimes it's better to keep it for yourself (if you are the block
					// producer), since for instance in some schemes others may copy your solution
					// and claim a reward.
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new pool
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn register_pool(
			origin: OriginFor<T>,
			name: [u8; 10],
			market: TradingPair,
			commission: u128,
			exit_fee: u128,
			public_funds_allowed: bool,
			trading_account: T::AccountId,
		) -> DispatchResult {
			let market_maker = ensure_signed(origin)?;

			ensure!(!<Pools<T>>::contains_key(market, &market_maker), Error::<T>::PoolExists);
			// Check market is active
			ensure!(T::OCEX::is_registered_market(&market), Error::<T>::UnknownMarket);
			// Check if commission and exit fee are between 0-1
			let mut commission =
				Decimal::from_u128(commission).ok_or(Error::<T>::ConversionError)?;
			let mut exit_fee = Decimal::from_u128(exit_fee).ok_or(Error::<T>::ConversionError)?;
			// Convert to Polkadex UNIT
			commission.div_assign(Decimal::from(UNIT_BALANCE));
			exit_fee.div_assign(Decimal::from(UNIT_BALANCE));
			ensure!(
				Decimal::zero() <= commission && commission <= Decimal::one(),
				Error::<T>::InvalidCommission
			);
			ensure!(
				Decimal::zero() <= exit_fee && exit_fee <= Decimal::one(),
				Error::<T>::InvalidExitFee
			);
			// Create the a pool address with origin and market combo if it doesn't exist
			let (pool, share_id) = Self::create_pool_account(&market_maker, market);
			T::OtherAssets::create(share_id, pool.clone(), true, One::one())?;
			// Transfer existential balance to pool id as fee, so that it never dies
			T::NativeCurrency::transfer(
				&market_maker,
				&pool,
				T::NativeCurrency::minimum_balance(),
				ExistenceRequirement::KeepAlive,
			)?;
			if let Some(base_asset) = market.base.asset_id() {
				T::OtherAssets::transfer(
					base_asset,
					&market_maker,
					&pool,
					T::OtherAssets::minimum_balance(base_asset),
					Preservation::Preserve,
				)?;
			}
			if let Some(quote_asset) = market.quote.asset_id() {
				T::OtherAssets::transfer(
					quote_asset,
					&market_maker,
					&pool,
					T::OtherAssets::minimum_balance(quote_asset),
					Preservation::Preserve,
				)?;
			}
			T::OtherAssets::transfer(
				market.quote.asset_id().ok_or(Error::<T>::ConversionError)?,
				&market_maker,
				&pool,
				T::OtherAssets::minimum_balance(
					market.quote.asset_id().ok_or(Error::<T>::ConversionError)?,
				),
				Preservation::Preserve,
			)?;
			// Register on OCEX pallet
			T::OCEX::register_pool(pool.clone(), trading_account)?;
			// Start cycle
			let config = MarketMakerConfig {
				pool_id: pool,
				commission,
				exit_fee,
				public_funds_allowed,
				name,
				share_id,
				force_closed: false,
			};
			<Pools<T>>::insert(market, market_maker, config);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			base_amount: u128,      // Amount of base asset to deposit
			max_quote_amount: u128, // Max quote amount willing to deposit
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;
			let config = <Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;
			ensure!(<SnapshotFlag<T>>::get().is_none(), Error::<T>::SnapshotInProgress); // TODO: @zktony Replace with pool level flags
			ensure!(!config.force_closed, Error::<T>::PoolForceClosed);
			if !config.public_funds_allowed && !config.force_closed {
				ensure!(lp == market_maker, Error::<T>::PublicDepositsNotAllowed);
			}

			let mut base_amount =
				Decimal::from_u128(base_amount).ok_or(Error::<T>::ConversionError)?;
			let mut max_quote_amount =
				Decimal::from_u128(max_quote_amount).ok_or(Error::<T>::ConversionError)?;
			// Convert to Polkadex UNIT
			base_amount.div_assign(Decimal::from(UNIT_BALANCE));
			max_quote_amount.div_assign(Decimal::from(UNIT_BALANCE));

			let average_price =
				T::OCEX::average_price(market).ok_or(Error::<T>::PriceNotAvailable)?;

			// Calculate the required quote asset
			let required_quote_amount = average_price.saturating_mul(base_amount);
			ensure!(required_quote_amount <= max_quote_amount, Error::<T>::NotEnoughQuoteAmount);
			Self::transfer_asset(&lp, &config.pool_id, base_amount, market.base)?;
			Self::transfer_asset(&lp, &config.pool_id, required_quote_amount, market.quote)?;
			let total_shares_issued = Decimal::from(
				T::OtherAssets::total_issuance(config.share_id).saturated_into::<u128>(),
			)
			.div(Decimal::from(UNIT_BALANCE));
			T::OCEX::add_liquidity(
				market,
				config.pool_id,
				lp,
				total_shares_issued,
				base_amount,
				required_quote_amount,
			)?;

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			shares: BalanceOf<T>,
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;

			let config = <Pools<T>>::get(market, market_maker).ok_or(Error::<T>::UnknownPool)?;
			ensure!(<SnapshotFlag<T>>::get().is_none(), Error::<T>::SnapshotInProgress); // TODO: @zktony Replace with pool level flags

			let total = T::OtherAssets::total_issuance(config.share_id);
			ensure!(!total.is_zero(), Error::<T>::TotalShareIssuanceIsZero);
			let burned_amt = T::OtherAssets::burn_from(
				config.share_id,
				&lp,
				shares,
				Precision::Exact,
				Fortitude::Polite,
			)?;
			// Queue it for execution at the end of the epoch
			let epoch = <WithdrawingEpoch<T>>::get();
			<WithdrawalRequests<T>>::mutate(epoch, config.pool_id, |pending| {
				pending.push((lp, burned_amt, total));
			});

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn force_close_pool(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(<Pools<T>>::contains_key(market, &market_maker), Error::<T>::UnknownPool);
			T::OCEX::force_close_pool(market, market_maker);
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn claim_rewards_by_lp(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			epoch: u16,
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;
			let pool_config =
				<Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;

			let total_rewards = match <Rewards<T>>::get(epoch, &pool_config.pool_id) {
				None => {
					let total_rewards =
						T::OCEX::claim_rewards(pool_config.pool_id.clone(), epoch, market)?;
					<Rewards<T>>::insert(epoch, pool_config.pool_id.clone(), total_rewards);
					total_rewards
				},
				Some(total_rewards) => total_rewards,
			};

			// Get the rewards for this LP after commission and exit fee
			let (mut scores_map, total_score, mm_claimed) =
				<LiquidityProviders<T>>::get(epoch, &pool_config.pool_id);

			let (score, already_claimed) =
				scores_map.get(&lp).ok_or(Error::<T>::InvalidLPAddress)?;
			if *already_claimed {
				return Err(Error::<T>::AlreadyClaimed.into());
			}
			let rewards_for_lp = score
				.saturating_mul(total_rewards)
				.checked_div(&total_score)
				.ok_or(Error::<T>::InvalidTotalScore)?;

			// Transfer it to LP's account
			T::NativeCurrency::transfer(
				&pool_config.pool_id,
				&lp,
				rewards_for_lp,
				ExistenceRequirement::AllowDeath,
			)?;
			scores_map.insert(lp, (*score, true));
			<LiquidityProviders<T>>::insert(
				epoch,
				pool_config.pool_id,
				(scores_map, total_score, mm_claimed),
			);
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn claim_rewards_by_mm(
			origin: OriginFor<T>,
			market: TradingPair,
			epoch: u16,
		) -> DispatchResult {
			let market_maker = ensure_signed(origin)?;
			let pool_config =
				<Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;

			let total_rewards = match <Rewards<T>>::get(epoch, &pool_config.pool_id) {
				None => {
					let total_rewards =
						T::OCEX::claim_rewards(pool_config.pool_id.clone(), epoch, market)?;
					<Rewards<T>>::insert(epoch, pool_config.pool_id.clone(), total_rewards);
					total_rewards
				},
				Some(total_rewards) => total_rewards,
			};

			// Get the rewards for this LP after commission and exit fee
			let (scores_map, total_score, already_claimed) =
				<LiquidityProviders<T>>::get(epoch, &pool_config.pool_id);
			if already_claimed {
				return Err(Error::<T>::AlreadyClaimed.into());
			}

			let rewards_for_mm = pool_config
				.commission
				.saturating_mul(
					Decimal::from(total_rewards.saturated_into::<u128>())
						.div(&Decimal::from(UNIT_BALANCE)),
				)
				.saturating_mul(Decimal::from(UNIT_BALANCE))
				.to_u128()
				.ok_or(Error::<T>::ConversionError)?
				.saturated_into();

			// Transfer it to LP's account
			T::NativeCurrency::transfer(
				&pool_config.pool_id,
				&market_maker,
				rewards_for_mm,
				ExistenceRequirement::AllowDeath,
			)?;

			<LiquidityProviders<T>>::insert(
				epoch,
				pool_config.pool_id,
				(scores_map, total_score, true),
			);
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn submit_scores_of_lps(
			origin: OriginFor<T>,
			results: LMPScoreSheet<T>,
		) -> DispatchResult {
			ensure_none(origin)?;

			for ((market, market_maker, epoch), (scores_map, total_score)) in results {
				let pool_config =
					<Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;
				<LiquidityProviders<T>>::insert(
					epoch,
					&pool_config.pool_id,
					(scores_map, total_score, false),
				);
				<Pools<T>>::insert(market, &market_maker, pool_config);
			}

			<SnapshotFlag<T>>::take();
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(10000)]
		// TODO: @zktony weight should be paramaterized by the number of requests and the market
		// maker is expected to call this multiple times to exhaust the pending withdrawals
		#[transactional]
		pub fn initiate_withdrawal(
			origin: OriginFor<T>,
			market: TradingPair,
			epoch: u16,
			num_requests: u16,
		) -> DispatchResult {
			let market_maker = ensure_signed(origin)?;
			let num_requests: usize = num_requests as usize;
			let pool_config =
				<Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;
			let mut requests = <WithdrawalRequests<T>>::get(epoch, &pool_config.pool_id);
			for request in requests.iter().take(num_requests) {
				T::OCEX::remove_liquidity(
					market,
					pool_config.pool_id.clone(),
					request.0.clone(),
					request.1,
					request.2,
				);
			}
			requests = requests[num_requests..].to_vec();
			<WithdrawalRequests<T>>::insert(epoch, pool_config.pool_id, requests);
			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn claim_force_closed_pool_funds(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;
			let pool_config =
				<Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;
			ensure!(pool_config.force_closed, Error::<T>::PoolNotForceClosed);
			// The system assumes all the base and quote funds in pool_id are claimed
			let lp_shares = T::OtherAssets::reducible_balance(
				pool_config.share_id,
				&lp,
				Preservation::Expendable,
				Fortitude::Force,
			);
			let total_issuance = T::OtherAssets::total_issuance(pool_config.share_id);

			let base_balance = T::OtherAssets::reducible_balance(
				market.base.asset_id().ok_or(Error::<T>::ConversionError)?,
				&pool_config.pool_id,
				Preservation::Expendable,
				Fortitude::Force,
			);

			let base_amt_to_claim = base_balance
				.saturating_mul(lp_shares)
				.checked_div(&total_issuance)
				.ok_or(Error::<T>::InvalidTotalIssuance)?;

			let quote_balance = T::OtherAssets::reducible_balance(
				market.base.asset_id().ok_or(Error::<T>::ConversionError)?,
				&pool_config.pool_id,
				Preservation::Expendable,
				Fortitude::Force,
			);

			let quote_amt_to_claim = quote_balance
				.saturating_mul(lp_shares)
				.checked_div(&total_issuance)
				.ok_or(Error::<T>::InvalidTotalIssuance)?;

			T::OtherAssets::burn_from(
				pool_config.share_id,
				&lp,
				lp_shares,
				Precision::Exact,
				Fortitude::Force,
			)?;
			T::OtherAssets::transfer(
				market.base.asset_id().ok_or(Error::<T>::ConversionError)?,
				&pool_config.pool_id,
				&lp,
				base_amt_to_claim,
				Preservation::Expendable,
			)?;
			T::OtherAssets::transfer(
				market.quote.asset_id().ok_or(Error::<T>::ConversionError)?,
				&pool_config.pool_id,
				&lp,
				quote_amt_to_claim,
				Preservation::Expendable,
			)?;
			// TODO: Emit events (Ask @frontend team about this)
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn take_snapshot() {
			let epoch = <LMPEpoch<T>>::get().saturating_sub(1); // We need to reduce the epoch by one
			let epoch_ending_blk = match <SnapshotFlag<T>>::get() {
				None => return,
				Some(blk) => blk,
			};
			// TODO: Only compute the result every five blocks

			let mut results: LMPScoreSheet<T> = BTreeMap::new();
			// Loop over all pools and lps and calculate score of all LPs
			for (market, mm, config) in <Pools<T>>::iter() {
				let mut scores_map = BTreeMap::new();
				let mut pool_total_score: BalanceOf<T> = Zero::zero();
				for (lp, mut total_shares) in <LPShares<T>>::iter_prefix(config.share_id) {
					let mut score: BalanceOf<T> = Zero::zero();
					let deposits_during_epoch =
						<AddLiquidityRecords<T>>::get(epoch, &(config.pool_id.clone(), lp.clone()));
					for (deposit_blk, share) in deposits_during_epoch {
						// Reduce share from total share to find the share deposited from previous
						// epoch
						total_shares = total_shares.saturating_sub(share);
						let diff = epoch_ending_blk.saturating_sub(deposit_blk);
						score =
							score
								.saturating_add(share.saturating_mul(
									diff.saturated_into::<u128>().saturated_into(),
								)); // Pro-rated scoring
					}
					score = score
						.saturating_add(total_shares.saturating_mul(201600u128.saturated_into())); // One epoch worth of blocks.
					scores_map.insert(lp, (score, false));
					pool_total_score = pool_total_score.saturating_add(score);
				}
				results.insert((market, mm, epoch), (scores_map, pool_total_score));
			}

			// Craft unsigned txn and send it.

			let call = Call::submit_scores_of_lps { results };

			match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()) {
				Ok(()) => {},
				Err(()) => {
					log::error!(target:"liquidity-mining","Unable to submit unsigned transaction");
				},
			}
		}

		pub fn create_pool_account(
			maker: &T::AccountId,
			market: TradingPair,
		) -> (T::AccountId, u128) {
			let mut preimage = Vec::new();
			maker.encode_to(&mut preimage);
			preimage.append(&mut market.encode());
			let hash = blake2_128(&preimage);
			let shares_id = u128::from_le_bytes(hash);
			let pool_id = T::PalletId::get();
			(pool_id.into_sub_account_truncating(hash), shares_id)
		}

		pub fn transfer_asset(
			payer: &T::AccountId,
			payee: &T::AccountId,
			mut amount: Decimal,
			asset: AssetId,
		) -> DispatchResult {
			amount.mul_assign(Decimal::from(UNIT_BALANCE));
			let amount: BalanceOf<T> =
				amount.to_u128().ok_or(Error::<T>::ConversionError)?.saturated_into();
			match asset {
				AssetId::Polkadex => {
					T::NativeCurrency::transfer(
						payer,
						payee,
						amount.unique_saturated_into(),
						ExistenceRequirement::KeepAlive,
					)?;
				},
				AssetId::Asset(id) => {
					T::OtherAssets::transfer(
						id,
						payer,
						payee,
						amount.unique_saturated_into(),
						Preservation::Preserve,
					)?;
				},
			}
			Ok(())
		}
	}
}
