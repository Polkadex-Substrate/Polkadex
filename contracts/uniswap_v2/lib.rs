#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;
use ink_storage::traits::{PackedLayout, SpreadLayout};
use scale::{Decode, Encode};
use sp_core::{H160};

#[derive(Debug, SpreadLayout, PackedLayout, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout))]
pub struct TradingPair(H160, H160);

#[ink::contract]
mod uniswap_v2 {

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct UniswapV2 {
        /// Stores a single `bool` value on the storage.
        liquidityPool: ink_storage::collections::HashMap<TradingPair, (Balance, Balance)>,
    }

    impl UniswapV2 {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        // #[ink(constructor)]
        // pub fn new(init_value: bool) -> Self {
        //     Self { value: init_value }
        // }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// A message that can be called on instantiated contracts.
        /// This one flips the value of the stored `bool` from `true`
        /// to `false` and vice versa.
        // #[ink(message)]
        // pub fn flip(&mut self) {
        //     self.value = !self.value;
        // }

        #[ink(message)]
        pub fn swap_with_exact_supply(
			origin: OriginFor<T>,
			path: Vec<CurrencyId>,
			#[pallet::compact] supply_amount: Balance,
			#[pallet::compact] min_target_amount: Balance,
		) {
			let who = ensure_signed(origin)?;
			Self::do_swap_with_exact_supply(&who, &path, supply_amount, min_target_amount, None)?;
		}

        #[ink(message)]
        pub fn swap_with_exact_target(
			origin: OriginFor<T>,
			path: Vec<CurrencyId>,
			#[pallet::compact] target_amount: Balance,
			#[pallet::compact] max_supply_amount: Balance,
		) {
			let who = ensure_signed(origin)?;
			Self::do_swap_with_exact_target(&who, &path, target_amount, max_supply_amount, None)?;
		}

        #[ink(message)]
        pub fn add_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			#[pallet::compact] max_amount_a: Balance,
			#[pallet::compact] max_amount_b: Balance,
			#[pallet::compact] min_share_increment: Balance,
			stake_increment_share: bool,
		) {
			let who = ensure_signed(origin)?;
			Self::do_add_liquidity(
				&who,
				currency_id_a,
				currency_id_b,
				max_amount_a,
				max_amount_b,
				min_share_increment,
				stake_increment_share,
			)?;
			Ok(().into())
		}

        #[ink(message)]
        pub fn remove_liquidity(
			origin: OriginFor<T>,
			currency_id_a: CurrencyId,
			currency_id_b: CurrencyId,
			#[pallet::compact] remove_share: Balance,
			#[pallet::compact] min_withdrawn_a: Balance,
			#[pallet::compact] min_withdrawn_b: Balance,
			by_unstake: bool,
		) {
			let who = ensure_signed(origin)?;
			Self::do_remove_liquidity(
				&who,
				currency_id_a,
				currency_id_b,
				remove_share,
				min_withdrawn_a,
				min_withdrawn_b,
				by_unstake,
			)?;
			Ok(().into())
		}

        fn do_add_liquidity(
            who: &T::AccountId,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            max_amount_a: Balance,
            max_amount_b: Balance,
            min_share_increment: Balance,
            stake_increment_share: bool,
        ) -> DispatchResult {
            let trading_pair =
                TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
            ensure!(
                matches!(
                    Self::trading_pair_statuses(trading_pair),
                    TradingPairStatus::<_, _>::Enabled
                ),
                Error::<T>::MustBeEnabled,
            );
    
            ensure!(
                !max_amount_a.is_zero() && !max_amount_b.is_zero(),
                Error::<T>::InvalidLiquidityIncrement
            );
    
            LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
                let dex_share_currency_id = trading_pair.dex_share_currency_id();
                let total_shares = T::Currency::total_issuance(dex_share_currency_id);
                let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.first() {
                    (max_amount_a, max_amount_b)
                } else {
                    (max_amount_b, max_amount_a)
                };
                let (pool_0_increment, pool_1_increment, share_increment): (Balance, Balance, Balance) =
                    if total_shares.is_zero() {
                        let (exchange_rate_0, exchange_rate_1) = if max_amount_0 > max_amount_1 {
                            (
                                ExchangeRate::one(),
                                ExchangeRate::checked_from_rational(max_amount_0, max_amount_1)
                                    .ok_or(ArithmeticError::Overflow)?,
                            )
                        } else {
                            (
                                ExchangeRate::checked_from_rational(max_amount_1, max_amount_0)
                                    .ok_or(ArithmeticError::Overflow)?,
                                ExchangeRate::one(),
                            )
                        };
    
                        let shares_from_token_0 = exchange_rate_0
                            .checked_mul_int(max_amount_0)
                            .ok_or(ArithmeticError::Overflow)?;
                        let shares_from_token_1 = exchange_rate_1
                            .checked_mul_int(max_amount_1)
                            .ok_or(ArithmeticError::Overflow)?;
                        let initial_shares = shares_from_token_0
                            .checked_add(shares_from_token_1)
                            .ok_or(ArithmeticError::Overflow)?;
    
                        (max_amount_0, max_amount_1, initial_shares)
                    } else {
                        let exchange_rate_0_1 =
                            ExchangeRate::checked_from_rational(*pool_1, *pool_0).ok_or(ArithmeticError::Overflow)?;
                        let input_exchange_rate_0_1 = ExchangeRate::checked_from_rational(max_amount_1, max_amount_0)
                            .ok_or(ArithmeticError::Overflow)?;
    
                        if input_exchange_rate_0_1 <= exchange_rate_0_1 {
                            // max_amount_0 may be too much, calculate the actual amount_0
                            let exchange_rate_1_0 =
                                ExchangeRate::checked_from_rational(*pool_0, *pool_1).ok_or(ArithmeticError::Overflow)?;
                            let amount_0 = exchange_rate_1_0
                                .checked_mul_int(max_amount_1)
                                .ok_or(ArithmeticError::Overflow)?;
                            let share_increment = Ratio::checked_from_rational(amount_0, *pool_0)
                                .and_then(|n| n.checked_mul_int(total_shares))
                                .ok_or(ArithmeticError::Overflow)?;
                            (amount_0, max_amount_1, share_increment)
                        } else {
                            // max_amount_1 is too much, calculate the actual amount_1
                            let amount_1 = exchange_rate_0_1
                                .checked_mul_int(max_amount_0)
                                .ok_or(ArithmeticError::Overflow)?;
                            let share_increment = Ratio::checked_from_rational(amount_1, *pool_1)
                                .and_then(|n| n.checked_mul_int(total_shares))
                                .ok_or(ArithmeticError::Overflow)?;
                            (max_amount_0, amount_1, share_increment)
                        }
                    };
    
                ensure!(
                    !share_increment.is_zero() && !pool_0_increment.is_zero() && !pool_1_increment.is_zero(),
                    Error::<T>::InvalidLiquidityIncrement,
                );
                ensure!(
                    share_increment >= min_share_increment,
                    Error::<T>::UnacceptableShareIncrement
                );
    
                let module_account_id = Self::account_id();
                T::Currency::transfer(trading_pair.first(), who, &module_account_id, pool_0_increment)?;
                T::Currency::transfer(trading_pair.second(), who, &module_account_id, pool_1_increment)?;
                T::Currency::deposit(dex_share_currency_id, who, share_increment)?;
    
                *pool_0 = pool_0.checked_add(pool_0_increment).ok_or(ArithmeticError::Overflow)?;
                *pool_1 = pool_1.checked_add(pool_1_increment).ok_or(ArithmeticError::Overflow)?;
    
                if stake_increment_share {
                    T::DEXIncentives::do_deposit_dex_share(who, dex_share_currency_id, share_increment)?;
                }
    
                Self::deposit_event(Event::AddLiquidity(
                    who.clone(),
                    trading_pair.first(),
                    pool_0_increment,
                    trading_pair.second(),
                    pool_1_increment,
                    share_increment,
                ));
                Ok(())
            })
        }
    
        fn do_remove_liquidity(
            who: &T::AccountId,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            remove_share: Balance,
            min_withdrawn_a: Balance,
            min_withdrawn_b: Balance,
            by_unstake: bool,
        ) {
            if remove_share.is_zero() {
                return Ok(());
            }
            let trading_pair =
                TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Error::<T>::InvalidCurrencyId)?;
            let dex_share_currency_id = trading_pair.dex_share_currency_id();
    
            LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> DispatchResult {
                let (min_withdrawn_0, min_withdrawn_1) = if currency_id_a == trading_pair.first() {
                    (min_withdrawn_a, min_withdrawn_b)
                } else {
                    (min_withdrawn_b, min_withdrawn_a)
                };
                let total_shares = T::Currency::total_issuance(dex_share_currency_id);
                let proportion =
                    Ratio::checked_from_rational(remove_share, total_shares).ok_or(ArithmeticError::Overflow)?;
                let pool_0_decrement = proportion.checked_mul_int(*pool_0).ok_or(ArithmeticError::Overflow)?;
                let pool_1_decrement = proportion.checked_mul_int(*pool_1).ok_or(ArithmeticError::Overflow)?;
                let module_account_id = Self::account_id();
    
                ensure!(
                    pool_0_decrement >= min_withdrawn_0 && pool_1_decrement >= min_withdrawn_1,
                    Error::<T>::UnacceptableLiquidityWithdrawn,
                );
    
                if by_unstake {
                    T::DEXIncentives::do_withdraw_dex_share(who, dex_share_currency_id, remove_share)?;
                }
                T::Currency::withdraw(dex_share_currency_id, &who, remove_share)?;
                T::Currency::transfer(trading_pair.first(), &module_account_id, &who, pool_0_decrement)?;
                T::Currency::transfer(trading_pair.second(), &module_account_id, &who, pool_1_decrement)?;
    
                *pool_0 = pool_0.checked_sub(pool_0_decrement).ok_or(ArithmeticError::Underflow)?;
                *pool_1 = pool_1.checked_sub(pool_1_decrement).ok_or(ArithmeticError::Underflow)?;
    
                Self::deposit_event(Event::RemoveLiquidity(
                    who.clone(),
                    trading_pair.first(),
                    pool_0_decrement,
                    trading_pair.second(),
                    pool_1_decrement,
                    remove_share,
                ));
                Ok(())
            })
        }
    
        fn get_liquidity(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
            if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b) {
                let (pool_0, pool_1) = Self::liquidity_pool(trading_pair);
                if currency_id_a == trading_pair.first() {
                    (pool_0, pool_1)
                } else {
                    (pool_1, pool_0)
                }
            } else {
                (Zero::zero(), Zero::zero())
            }
        }
    

        /// Simply returns the current value of our `bool`.
        // #[ink(message)]
        // pub fn get(&self) -> bool {
        //     self.value
        // }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            let uniswap_v2 = UniswapV2::default();
            assert_eq!(uniswap_v2.get(), false);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn it_works() {
            let mut uniswap_v2 = UniswapV2::new(false);
            assert_eq!(uniswap_v2.get(), false);
            uniswap_v2.flip();
            assert_eq!(uniswap_v2.get(), true);
        }
    }
}
