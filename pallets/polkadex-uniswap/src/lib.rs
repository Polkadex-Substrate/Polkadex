#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, sp_std};
use frame_support::dispatch::DispatchResult;

use frame_support::sp_std::convert::TryInto;

use frame_support::traits::{ExistenceRequirement, Get};
use frame_system::ensure_signed;
use sp_arithmetic::FixedPointNumber;
use sp_arithmetic::traits::{CheckedDiv, CheckedMul, UniqueSaturatedFrom};
use sp_std::vec::Vec;
use sp_runtime::{FixedU128, ModuleId};
use sp_std::vec;
use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};



#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + polkadex_custom_assets::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Maximum Trading Path limit
    type TradingPathLimit: Get<usize>;
}



decl_storage! {

	trait Store for Module<T: Config> as PolkadexSwapEngine {
	    /// Liquidity pool for specific pair(a tuple consisting of two sorted AssetIds).
		/// (AssetID, AssetID) -> (Amount_0, Amount_1, Total LPShares)
		LiquidityPool get(fn liquidity_pool): map hasher(twox_64_concat) (T::Hash,T::Hash) => (FixedU128, FixedU128, FixedU128);
		/// LPShare holdings
		LiquidityPoolHoldings get(fn holdings): map hasher(identity) (T::AccountId,(T::Hash,T::Hash)) => FixedU128;
		/// Swapping Fee
		SwappingFee: FixedU128 = FixedU128::from(3)/FixedU128::from(1000);
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Config>::AccountId,
		AssetId = <T as frame_system::Config>::Hash
	{
		/// Add liquidity success. \[who, currency_id_0, pool_0_increment, currency_id_1, pool_1_increment, share_increment\]
		AddLiquidity(AccountId, AssetId, FixedU128, AssetId, FixedU128, FixedU128),
		/// Remove liquidity from the trading pool success. \[who, currency_id_0, pool_0_decrement, currency_id_1, pool_1_decrement, share_decrement\]
		RemoveLiquidity(AccountId, AssetId, FixedU128, AssetId, FixedU128, FixedU128),
		/// Use supply currency to swap target currency. \[trader, trading_path, supply_currency_amount, target_currency_amount\]
		Swap(AccountId, Vec<AssetId>, FixedU128, FixedU128),
	}
);

decl_error! {
	/// Error for dex module.
	pub enum Error for Module<T: Config> {
		/// Not the enable trading pair
		TradingPairNotAllowed,
		/// The increment of liquidity is invalid
		InvalidLiquidityIncrement,
		/// Invalid currency id
		InvalidCurrencyId,
		/// Invalid trading path length
		InvalidTradingPathLength,
		/// Target amount is less to min_target_amount
		InsufficientTargetAmount,
		/// Supply amount is more than max_supply_amount
		ExcessiveSupplyAmount,
		/// The swap will cause unacceptable price impact
		ExceedPriceImpactLimit,
		/// Liquidity is not enough
		InsufficientLiquidity,
		/// The supply amount is zero
		ZeroSupplyAmount,
		/// The target amount is zero
		ZeroTargetAmount,
		/// Failed to convert T::Balance to FixedU128
		FixedU128ConversionFailed,
		///ProvidedAmountIsZero
		ProvidedAmountIsZero,
		///Insufficent Balance
		InsufficientBalance,
		///LowShare
		LowShare,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

        /// The limit for length of trading path
		const TradingPathLimit: u32 = T::TradingPathLimit::get() as u32;

        /// This method registers new Swap Pair and insert liquidity.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `currency_id_a` - Currency Id of Counter Asset.
        ///
        /// * `currency_id_b` - Currency Id of Base Asset.
        ///
        /// * `currency_id_a_amount` - Balance provided by Trader for Counter Asset.
        ///
        /// * `currency_id_b_amount` - Balance provided by Trader for Base Asset.
        ///
        /// # Return
        ///
        ///  This function returns a status that, new Swap Pair is successfully registered or not.

        #[weight=10000]
        pub fn register_swap_pair(origin, currency_id_a: T::Hash, currency_id_b: T::Hash, currency_id_a_amount: T::Balance,
                                    currency_id_b_amount: T::Balance) -> dispatch::DispatchResult{
             let who = ensure_signed(origin)?;
             Self::do_register_swap_pair(&who,currency_id_a,currency_id_b,currency_id_a_amount,currency_id_b_amount)?;
             Ok(())
        }

		/// This method swap supply amount for amount less then target value.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `path` - Trading Path.
        ///
        /// * `supply_amount` - Provided amount to Swap.
        ///
        /// * `min_target_amount` - Acceptable minimum target amount.
        ///
        /// # Return
        ///
        ///  This function returns a status that, new Swap successfully happened or not.

		#[weight = 10000]
		pub fn swap_with_exact_supply(origin, path: Vec<T::Hash>, #[compact] supply_amount: T::Balance, #[compact] min_target_amount: T::Balance) -> dispatch::DispatchResult{
				let who = ensure_signed(origin)?;
				Self::do_swap_with_exact_supply(&who, &path, supply_amount, min_target_amount,None)?;
				Ok(())
		}

		/// This Method swaps with Exact target amount.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `path` - Trading Path.
        ///
        /// * `target_amount` - Provided target amount for exact Swap.
        ///
        /// * `max_supply_amount` - Acceptable maximum supply amount.
        ///
        /// # Return
        ///
        ///  This function returns a status that, new Swap successfully happened or not.
		#[weight = 10000]
		pub fn swap_with_exact_target(origin, path: Vec<T::Hash>, #[compact] target_amount: T::Balance, #[compact] max_supply_amount: T::Balance) -> dispatch::DispatchResult{
				let who = ensure_signed(origin)?;
				Self::do_swap_with_exact_target(&who, &path, target_amount, max_supply_amount,None)?;
				Ok(())
		}

		/// This Method injects Liquidity to Specific Liquidity pool.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `currency_id_a` - Currency Id of Counter Asset.
        ///
        /// * `currency_id_b` - Currency Id of Base Asset.
        ///
        /// * `max_amount_a` - Maximum Counter Asset Id's amount allowed to inject to liquidity pool.
        ///
        /// * `max_amount_b` - Maximum Counter Base Id's amount allowed to inject to liquidity pool.
        ///
        /// # Return
        ///
        ///  This function returns a status that, Liquidity is successfully inserted or not.

		#[weight = 10000]
		pub fn add_liquidity(origin, currency_id_a: T::Hash, currency_id_b: T::Hash,
		                    #[compact] max_amount_a: T::Balance, #[compact] max_amount_b: T::Balance) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_add_liquidity(&who, currency_id_a, currency_id_b, max_amount_a, max_amount_b)?;
			Ok(())
		}

        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `currency_id_a` - Currency Id of Counter Asset.
        ///
        /// * `currency_id_b` - Currency Id of Base Asset.
        ///
        /// * `remove_share` - Liquidity amount to remove.
        ///
        /// # Return
        ///
        ///  This function returns a status that, Liquidity is successfully removed or not.

		#[weight = 10000]
		pub fn remove_liquidity(origin, currency_id_a: T::Hash, currency_id_b: T::Hash, #[compact] remove_share: T::Balance) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_remove_liquidity(&who, currency_id_a, currency_id_b, remove_share)?;
			Ok(())
		}
	}
}


impl<T: Config> Module<T> {
    /// Stores all the assets related to Swap
    pub fn get_wallet_account() -> T::AccountId {
        ModuleId(*b"pswapacc").into_account()
    }

    /// Registers new Swap Pair and insert liquidity.
    pub fn do_register_swap_pair(who: &T::AccountId, currency_id_a: T::Hash, currency_id_b: T::Hash, currency_id_a_amount: T::Balance, currency_id_b_amount: T::Balance) -> DispatchResult {
        ensure!(!currency_id_a_amount.is_zero() && !currency_id_b_amount.is_zero(),Error::<T>::InsufficientLiquidity);
        let trading_pair = Self::get_pair(currency_id_a, currency_id_b);
        ensure!(!<LiquidityPool<T>>::contains_key(&trading_pair), Error::<T>::TradingPairNotAllowed);
        Self::do_add_liquidity(who, currency_id_a, currency_id_b, currency_id_a_amount, currency_id_b_amount)
    }

    /// Swaps supply amount for amount less then Minimum target amount.
    pub fn do_swap_with_exact_supply(who: &T::AccountId, path: &Vec<T::Hash>, supply_amount: T::Balance, min_target_amount: T::Balance, price_impact_limit: Option<FixedU128>) -> DispatchResult {
        let supply_amount: FixedU128 = Self::convert_balance_to_fixedU128(supply_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        let min_target_amount: FixedU128 = Self::convert_balance_to_fixedU128(min_target_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;

        Self::do_swap_with_exact_supply_fixedu128(who, path, supply_amount, min_target_amount, price_impact_limit)
    }

    ///  Swaps supply amount for amount less then minimum target amount (FixedU128).
    pub fn do_swap_with_exact_supply_fixedu128(who: &T::AccountId, path: &Vec<T::Hash>, supply_amount: FixedU128, min_target_amount: FixedU128, price_impact_limit: Option<FixedU128>) -> DispatchResult {
        let amounts = Self::get_target_amounts(&path, supply_amount, price_impact_limit)?;
        ensure!(amounts[amounts.len() - 1] >= min_target_amount, Error::<T>::InsufficientTargetAmount);
        let module_account_id = Self::get_wallet_account();

        let actual_target_amount = amounts[amounts.len() - 1];

        // TODO: Remove this dirty hacks
        let actual_target_amount_converted = Self::convert_fixedU128_to_balance(actual_target_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        let supply_amount_converted = Self::convert_fixedU128_to_balance(supply_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        // TODO: @Krishna Please take care of results from the transfers, ensure it's not error
        polkadex_custom_assets::Module::<T>::transfer(who, &module_account_id, path[0], &supply_amount_converted, ExistenceRequirement::AllowDeath)?;
        Self::_swap_by_path(&path, &amounts);
        polkadex_custom_assets::Module::<T>::transfer(&module_account_id, who, path[path.len() - 1], &actual_target_amount_converted, ExistenceRequirement::AllowDeath)?;

        Self::deposit_event(RawEvent::Swap(who.clone(), path.to_vec(), supply_amount, actual_target_amount));

        Ok(())
    }
    /// Swaps with Exact target amount.
    pub fn do_swap_with_exact_target(who: &T::AccountId, path: &Vec<T::Hash>, target_amount: T::Balance, max_supply_amount: T::Balance, price_impact_limit: Option<FixedU128>) -> DispatchResult {
        let target_amount: FixedU128 = Self::convert_balance_to_fixedU128(target_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        let max_supply_amount: FixedU128 = Self::convert_balance_to_fixedU128(max_supply_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;


        let amounts = Self::get_supply_amounts(&path, target_amount, price_impact_limit)?;
        ensure!(amounts[0] <= max_supply_amount, Error::<T>::ExcessiveSupplyAmount);
        let module_account_id = Self::get_wallet_account();
        let actual_supply_amount = amounts[0];

        // TODO: Remove this dirty hacks
        let actual_supply_amount_converted = Self::convert_fixedU128_to_balance(actual_supply_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        let target_amount_converted = Self::convert_fixedU128_to_balance(target_amount).ok_or(Error::<T>::FixedU128ConversionFailed)?;

        // TODO: @Krishna Please take care of results from the transfers, ensure it's not error
        polkadex_custom_assets::Module::<T>::transfer(who, &module_account_id, path[0], &actual_supply_amount_converted, ExistenceRequirement::AllowDeath)?;
        Self::_swap_by_path(&path, &amounts);
        polkadex_custom_assets::Module::<T>::transfer(&module_account_id, who, path[path.len() - 1], &target_amount_converted, ExistenceRequirement::AllowDeath)?;

        Self::deposit_event(RawEvent::Swap(who.clone(), path.to_vec(), actual_supply_amount, target_amount));
        Ok(())
    }

    pub fn get_liquidity(currency_id_a: T::Hash, currency_id_b: T::Hash) -> (FixedU128, FixedU128) {
        let trading_pair = Self::get_pair(currency_id_a, currency_id_b);
        let (pool_0, pool_1, _) = Self::liquidity_pool(trading_pair);
        if currency_id_a == trading_pair.0 {
            (pool_0, pool_1)
        } else {
            (pool_1, pool_0)
        }
    }

    /// Get how much target amount will be got for specific supply amount and price impact.
    fn get_target_amount(supply_pool: FixedU128, target_pool: FixedU128, supply_amount: FixedU128) -> FixedU128 {
        if supply_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
            FixedU128::zero()
        } else {
            let swap_fee: FixedU128 = SwappingFee::get();
            let fee_term: FixedU128 = FixedU128::from(1).saturating_sub(swap_fee);

            let fee_reduced_supply_amount: FixedU128 = supply_amount.saturating_mul(fee_term);

            let numerator: FixedU128 = target_pool.saturating_mul(fee_reduced_supply_amount);  // product makes this value too low


            let denominator: FixedU128 = supply_pool.saturating_add(fee_reduced_supply_amount);

            let target_amount: FixedU128 = numerator.checked_div(&denominator)
                .unwrap_or_else(FixedU128::zero);

            target_amount
        }
    }

    /// Get supply amount paid for specific target amount.
    fn get_supply_amount(supply_pool: FixedU128, target_pool: FixedU128, target_amount: FixedU128) -> FixedU128 {
        if target_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
            FixedU128::zero()
        } else {
            let swap_fee: FixedU128 = SwappingFee::get();
            let numerator: FixedU128 = target_amount.saturating_mul(supply_pool);
            let fee_term: FixedU128 = FixedU128::from(1).saturating_sub(swap_fee);
            let sub: FixedU128 = target_pool.saturating_sub(target_amount);
            let denominator: FixedU128 = sub.saturating_mul(fee_term);

            let supply_amount: FixedU128 = numerator.checked_div(&denominator).unwrap_or_else(FixedU128::zero);
            supply_amount
        }
    }
    /// Get vector of target amount for specific supply amount and price impact.
    fn get_target_amounts(path: &[T::Hash], supply_amount: FixedU128, price_impact_limit: Option<FixedU128>) -> sp_std::result::Result<Vec<FixedU128>, Error<T>> {
        let path_length = path.len();
        ensure!(path_length >= 2 && path_length <= T::TradingPathLimit::get(), Error::<T>::InvalidTradingPathLength);
        let mut target_amounts: Vec<FixedU128> = vec![FixedU128::zero(); path_length];
        target_amounts[0] = supply_amount;

        let mut i: usize = 0;
        while i + 1 < path_length {
            ensure!(LiquidityPool::<T>::contains_key(Self::get_pair(path[i],path[i+1])),Error::<T>::TradingPairNotAllowed);
            let (supply_pool, target_pool) = Self::get_liquidity(path[i], path[i + 1]);
            ensure!(!supply_pool.is_zero() && !target_pool.is_zero(),Error::<T>::InsufficientLiquidity);
            let target_amount = Self::get_target_amount(supply_pool, target_pool, target_amounts[i]);
            ensure!(!target_amount.is_zero(), Error::<T>::ZeroTargetAmount);

            // check price impact if limit exists
            if let Some(limit) = price_impact_limit {
                let price_impact = target_amount.checked_div(&target_pool).unwrap_or_else(FixedU128::zero);
                ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
            }

            target_amounts[i + 1] = target_amount;
            i += 1;
        }

        Ok(target_amounts)
    }
    /// Get vector of supply amount for specific target amount and price impact.
    fn get_supply_amounts(path: &[T::Hash], target_amount: FixedU128, price_impact_limit: Option<FixedU128>) -> sp_std::result::Result<Vec<FixedU128>, Error<T>> {
        let path_length = path.len();
        ensure!(path_length >= 2 && path_length <= T::TradingPathLimit::get(), Error::<T>::InvalidTradingPathLength);

        let mut supply_amounts: Vec<FixedU128> = vec![FixedU128::zero(); path_length];
        supply_amounts[path_length - 1] = target_amount;

        let mut i: usize = path_length - 1;
        while i > 0 {
            ensure!(LiquidityPool::<T>::contains_key(Self::get_pair(path[i-1],path[i])), Error::<T>::TradingPairNotAllowed);
            let (supply_pool, target_pool) = Self::get_liquidity(path[i - 1], path[i]);
            ensure!(!supply_pool.is_zero() && !target_pool.is_zero(),Error::<T>::InsufficientLiquidity);

            let supply_amount = Self::get_supply_amount(supply_pool, target_pool, supply_amounts[i]);
            ensure!(!supply_amount.is_zero(), Error::<T>::ZeroSupplyAmount);

            // check price impact if limit exists
            if let Some(limit) = price_impact_limit {
                let price_impact = supply_amounts[i].checked_div(&target_pool).unwrap_or_else(FixedU128::zero);
                ensure!(price_impact <= limit, Error::<T>::ExceedPriceImpactLimit);
            };

            supply_amounts[i - 1] = supply_amount;
            i -= 1;
        }

        Ok(supply_amounts)
    }

    fn _swap(supply_currency_id: T::Hash, target_currency_id: T::Hash, supply_increment: FixedU128, target_decrement: FixedU128) {
        let trading_pair = Self::get_pair(supply_currency_id, target_currency_id);
        LiquidityPool::<T>::mutate(trading_pair, |(pool_0, pool_1, _pool_shares): &mut (FixedU128, FixedU128, FixedU128)| {
            if supply_currency_id == trading_pair.0 {
                *pool_0 = pool_0.saturating_add(supply_increment);
                *pool_1 = pool_1.saturating_sub(target_decrement);
            } else {
                *pool_0 = pool_0.saturating_sub(target_decrement);
                *pool_1 = pool_1.saturating_add(supply_increment);
            }
        });
    }

    fn _swap_by_path(path: &[T::Hash], amounts: &[FixedU128]) {
        let mut i: usize = 0;
        while i + 1 < path.len() {
            let (supply_currency_id, target_currency_id) = (path[i], path[i + 1]);
            let (supply_increment, target_decrement) = (amounts[i], amounts[i + 1]);
            Self::_swap(
                supply_currency_id,
                target_currency_id,
                supply_increment,
                target_decrement,
            );
            i += 1;
        }
    }
    /// Adds Liquidity for specific swapping pair.
    pub fn do_add_liquidity(who: &T::AccountId, currency_id_a: T::Hash, currency_id_b: T::Hash, max_amount_a: T::Balance, max_amount_b: T::Balance) -> dispatch::DispatchResult {
        ensure!(!max_amount_a.is_zero() && !max_amount_b.is_zero(), Error::<T>::ProvidedAmountIsZero);
        let max_amount_a: FixedU128 = Self::convert_balance_to_fixedU128(max_amount_a).ok_or(Error::<T>::FixedU128ConversionFailed)?;
        let max_amount_b: FixedU128 = Self::convert_balance_to_fixedU128(max_amount_b).ok_or(Error::<T>::FixedU128ConversionFailed)?;

        let trading_pair = Self::get_pair(currency_id_a, currency_id_b);

        <LiquidityPool<T>>::try_mutate(trading_pair, |(pool_0, pool_1, pool_shares)| -> dispatch::DispatchResult {
            let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.0 {
                (max_amount_a, max_amount_b)
            } else {
                (max_amount_b, max_amount_a)
            };

            let (pool_0_increment, pool_1_increment, share_increment): (FixedU128, FixedU128, FixedU128) =
                if FixedU128::is_zero(pool_shares) {
                    // initialize this liquidity pool, the initial share is equal to the max value
                    // between base currency amount and other currency amount
                    let initial_share = sp_std::cmp::max(max_amount_0, max_amount_1);
                    (max_amount_0, max_amount_1, initial_share)
                } else {
                    let price_0_1 = pool_1.checked_div(&pool_0).unwrap_or(FixedU128::zero());
                    let input_price_0_1 = max_amount_1.checked_div(&max_amount_0).unwrap_or(FixedU128::zero());

                    if input_price_0_1 <= price_0_1 {
                        // max_amount_0 may be too much, calculate the actual amount_0
                        let price_1_0: FixedU128 = pool_0.checked_div(pool_1).unwrap_or(FixedU128::zero());
                        let amount_0 = price_1_0.checked_mul(&max_amount_1).unwrap_or(FixedU128::zero());
                        let share_increment = amount_0.checked_div(pool_0).unwrap_or(FixedU128::zero())
                            .checked_mul(pool_shares).unwrap_or(FixedU128::zero());
                        (amount_0, max_amount_1, share_increment)
                    } else {
                        // max_amount_1 is too much, calculate the actual amount_1
                        let amount_1 = price_0_1.checked_mul(&max_amount_0).unwrap_or(FixedU128::zero());
                        let share_increment = amount_1.checked_div(pool_1).unwrap_or(FixedU128::zero())
                            .checked_mul(pool_shares)
                            .unwrap_or(FixedU128::zero());
                        (max_amount_0, amount_1, share_increment)
                    }
                };
            ensure!(!share_increment.is_zero() && !pool_0_increment.is_zero() && !pool_1_increment.is_zero(), Error::<T>::InvalidLiquidityIncrement);
            let swap_wallet_account = Self::get_wallet_account();
            // TODO: Dirty Hack
            let pool_0_increment_converted = Self::convert_fixedU128_to_balance(pool_0_increment).ok_or(Error::<T>::FixedU128ConversionFailed)?;
            let pool_1_increment_converted = Self::convert_fixedU128_to_balance(pool_1_increment).ok_or(Error::<T>::FixedU128ConversionFailed)?;
            polkadex_custom_assets::Module::<T>::transfer(who, &swap_wallet_account, trading_pair.0, &pool_0_increment_converted, ExistenceRequirement::AllowDeath)?;
            polkadex_custom_assets::Module::<T>::transfer(who, &swap_wallet_account, trading_pair.1, &pool_1_increment_converted, ExistenceRequirement::AllowDeath)?;

            <LiquidityPoolHoldings<T>>::try_mutate((who, trading_pair), |lp_shares| -> dispatch::DispatchResult {
                *lp_shares = lp_shares.saturating_add(share_increment);
                Ok(())
            })?;

            *pool_0 = pool_0.saturating_add(pool_0_increment);
            *pool_1 = pool_1.saturating_add(pool_1_increment);
            *pool_shares = pool_shares.saturating_add(share_increment); // TODO ask @gautham about this

            Self::deposit_event(RawEvent::AddLiquidity(
                who.clone(),
                trading_pair.0,
                pool_0_increment,
                trading_pair.1,
                pool_1_increment,
                share_increment,
            ));
            Ok(())
        })
    }
    /// Removes liquidity for specific trading pair.
    pub fn do_remove_liquidity(who: &T::AccountId, currency_id_a: T::Hash, currency_id_b: T::Hash, remove_share: T::Balance) -> DispatchResult {
        if remove_share.is_zero() {
            return Ok(());
        }
        let remove_share: FixedU128 = Self::convert_balance_to_fixedU128(remove_share).ok_or(Error::<T>::FixedU128ConversionFailed)?;

        let trading_pair = Self::get_pair(currency_id_a, currency_id_b);
        ensure!(<LiquidityPool<T>>::contains_key(&trading_pair), Error::<T>::TradingPairNotAllowed);
        let original_share = <LiquidityPoolHoldings<T>>::get((who, trading_pair));
        ensure!(remove_share <= original_share, Error::<T>::LowShare);

        <LiquidityPool<T>>::try_mutate(trading_pair, |(pool_0, pool_1, pool_shares)| -> dispatch::DispatchResult {
            let proportion = remove_share.checked_div(pool_shares).unwrap_or(FixedU128::zero());
            let pool_0_decrement = proportion.saturating_mul(*pool_0);
            let pool_1_decrement = proportion.saturating_mul(*pool_1);
            let swap_wallet_account = Self::get_wallet_account();


            // TODO: Dirty Hack
            let pool_0_decrement_converted = Self::convert_fixedU128_to_balance(pool_0_decrement).ok_or(Error::<T>::FixedU128ConversionFailed)?;
            let pool_1_decrement_converted = Self::convert_fixedU128_to_balance(pool_1_decrement).ok_or(Error::<T>::FixedU128ConversionFailed)?;

            polkadex_custom_assets::Module::<T>::transfer(&swap_wallet_account, &who, trading_pair.0, &pool_0_decrement_converted, ExistenceRequirement::KeepAlive)?;
            polkadex_custom_assets::Module::<T>::transfer(&swap_wallet_account, &who, trading_pair.1, &pool_1_decrement_converted, ExistenceRequirement::KeepAlive)?;

            *pool_0 = pool_0.saturating_sub(pool_0_decrement);
            *pool_1 = pool_1.saturating_sub(pool_1_decrement);

            <LiquidityPoolHoldings<T>>::try_mutate((who, trading_pair), |lp_shares| -> dispatch::DispatchResult {
                *lp_shares = lp_shares.saturating_sub(remove_share);
                Ok(())
            })?;


            Self::deposit_event(RawEvent::RemoveLiquidity(
                who.clone(),
                trading_pair.0,
                pool_0_decrement,
                trading_pair.1,
                pool_1_decrement,
                remove_share,
            ));
            Ok(())
        })
    }

    fn get_pair(currency_id_a: T::Hash, currency_id_b: T::Hash) -> (T::Hash, T::Hash) {
        if currency_id_a > currency_id_b {
            (currency_id_a, currency_id_b)
        } else {
            (currency_id_b, currency_id_a)
        }
    }

    #[allow(non_snake_case)]
    pub fn convert_balance_to_fixedU128(amount: T::Balance) -> Option<FixedU128> {
        if let Some(y) = TryInto::<u128>::try_into(amount).ok() {
            FixedU128::from(y).checked_div(&FixedU128::from(1_000_000_000_000))
        } else {
            None
        }
    }

    #[allow(non_snake_case)]
    pub fn convert_fixedU128_to_balance(x: FixedU128) -> Option<T::Balance> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(UniqueSaturatedFrom::<u128>::unique_saturated_from(balance_in_u128))
        } else {
            None
        }
    }

    /// Gives best deal.
    fn give_best_deal(max_supply: FixedU128, min_target: FixedU128, trading_pair: (T::Hash, T::Hash)) -> Option<(FixedU128, FixedU128)> {
        let path = vec![trading_pair.0, trading_pair.1];
        let target = Self::get_target_amounts(&path, max_supply, None).ok();
        let target_vector = &target.clone().unwrap(); // TODO: Remove unwrap


        match target {
            Some(target_vector) => {
                if target_vector[target_vector.len() - 1] >= min_target {
                    Some((max_supply, target_vector[target_vector.len() - 1]))
                } else {
                    None
                }
            }
            None => None
        }
    }
    /// Executes given deal.
    fn execute_deal(who: &T::AccountId, optimal_deal: (FixedU128, FixedU128), trading_pair: (T::Hash, T::Hash)) -> bool {

        let path = vec![trading_pair.0, trading_pair.1];

        match Self::do_swap_with_exact_supply_fixedu128(&who, &path, optimal_deal.0, optimal_deal.1, None) {
            Ok(_) => true,
            Err(_) => {false}
        }
    }
    /// Executes deal if best deal
    pub fn swap_by_orderbook(who: &T::AccountId, trading_pair: (T::Hash, T::Hash), is_bidlimit: bool, price: FixedU128, quantity: FixedU128, quote_asset_id: T::Hash) -> bool {
        let balance_tuple: (FixedU128, FixedU128) = if is_bidlimit { (price.saturating_mul(quantity), quantity) } else { (quantity, price.saturating_mul(quantity)) };
        let path: (T::Hash, T::Hash) = match is_bidlimit {
            true if quote_asset_id == trading_pair.0 => (trading_pair.1, trading_pair.0),
            false if quote_asset_id == trading_pair.0 => trading_pair,
            true if quote_asset_id == trading_pair.1 => trading_pair,
            false if quote_asset_id == trading_pair.1 => (trading_pair.1, trading_pair.0),
            _ => { return false; }
        };
        let (max_supply, min_target) = balance_tuple;
        let temp = match Self::give_best_deal(max_supply, min_target, path) {
            Some(optimal_deal) => Self::execute_deal(who, optimal_deal, path),
            None => false,
        };
        temp
    }
}
