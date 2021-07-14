#![cfg_attr(not(feature = "std"), no_std)]

mod models;

use ink_lang as ink;

/// Error types
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, err_derive::Error)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    #[error(display = "CurrencyId is invalid")]
    InvalidCurrencyId,
    #[error(display = "LiquidityIncrement is invalid")]
    InvalidLiquidityIncrement,
    #[error(display = "Arithmetic Overflow occured")]
    ArithmeticOverflow,
    #[error(display = "Share Increment is invalid")]
    UnacceptableShareIncrement,
    #[error(display = "Unacceptable Liqudity withdrawn")]
    UnacceptableLiquidityWithdrawn,
}

pub type Result<T> = core::result::Result<T, Error>;

#[ink::contract]
mod uniswap_v2 {
    use super::*;
    use crate::{
        models::{TradingPair, TokenAddress, ExchangeRate},
    };
    use ink_storage::{
        collections::HashMap
    };

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct UniswapV2 {
        /// Stores a liquidityPool hashmap value on the storage.
        liquidityPool: HashMap<TradingPair, (Balance, Balance)>,
        totalIssuances: HashMap<TradingPair, Balance>,
    }

    /// Emitted when Adding liquidity success. \[who, currency_id_0, pool_0_increment, currency_id_1, pool_1_increment, share_increment\].
    #[ink(event)]
    pub struct LiquidityAdded {
        who: T::AccountId,
        currency_id_0: TokenAddress,
        pool_0_increment: Balance,
        currency_id_1: CurrencyId,
        pool_1_increment: Balance,
        share_increment: Balance,
    }

    /// Emitted when Removing liquidity from the trading pool success. \[who, currency_id_0, pool_0_decrement, currency_id_1, pool_1_decrement, share_decrement\]
    #[ink(event)]
    pub struct LiquidityRemoved {
        who: T::AccountId,
        currency_id_0: TokenAddress,
        pool_0_increment: Balance,
        currency_id_1: CurrencyId,
        pool_1_increment: Balance,
        share_increment: Balance,
    }

    impl UniswapV2 {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                liquidityPool: Default::default()
            }
        }

        /// Add liquidity to trading pair
        /// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		/// - `max_amount_a`: maximum amount of currency_id_a is allowed to inject to liquidity
		///   pool.
		/// - `max_amount_b`: maximum amount of currency_id_b is allowed to inject to liquidity
		///   pool.
		/// - `min_share_increment`: minimum acceptable share amount.
		/// - `stake_increment_share`: indicates whether to stake increased dex share to earn
		///   incentives
        #[ink(message)]
        pub fn add_liquidity(
            &self,
			currency_id_a: TokenAddress,
			currency_id_b: TokenAddress,
			max_amount_a: Balance,
			max_amount_b: Balance,
			min_share_increment: Balance,
			stake_increment_share: bool,
		) -> Result<()> {
			let caller = self.env().caller();
			self.do_add_liquidity(
				&caller,
				currency_id_a,
				currency_id_b,
				max_amount_a,
				max_amount_b,
				min_share_increment,
				stake_increment_share,
			)?;
			Ok(())
		}

        /// Remove liquidity from specific liquidity pool in the form of burning
		/// shares, and withdrawing currencies in trading pairs from liquidity
		/// pool in proportion, and withdraw liquidity incentive interest.
		///
		/// - `currency_id_a`: currency id A.
		/// - `currency_id_b`: currency id B.
		/// - `remove_share`: liquidity amount to remove.
		/// - `min_withdrawn_a`: minimum acceptable withrawn for currency_id_a.
		/// - `min_withdrawn_b`: minimum acceptable withrawn for currency_id_b.
		/// - `by_unstake`: this flag indicates whether to withdraw share which is on incentives.
        #[ink(message)]
        pub fn remove_liquidity(
            &self,
			currency_id_a: TokenAddress,
			currency_id_b: TokenAddress,
			remove_share: Balance,
			min_withdrawn_a: Balance,
			min_withdrawn_b: Balance,
			by_unstake: bool,
		) -> Result<()> {
			let caller = self.env().caller();
			Self::do_remove_liquidity(
				&caller,
				currency_id_a,
				currency_id_b,
				remove_share,
				min_withdrawn_a,
				min_withdrawn_b,
				by_unstake,
			)?;
			Ok(())
		}

        /// Transfers token `id` `from` the sender to the `to` AccountId.
        fn do_add_liquidity(
            &mut self,
            currency_id_a: CurrencyId,
            currency_id_b: CurrencyId,
            max_amount_a: Balance,
            max_amount_b: Balance,
            min_share_increment: Balance,
            stake_increment_share: bool,
        ) -> Result<()> {
            let caller = self.env().caller();

            let trading_pair = TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Err(Error::InvalidCurrencyId))?;

            if max_amount_a.is_zero() || max_amount_b.is_zero() {
                return Err(Error::InvalidLiquidityIncrement)
            }

            LiquidityPool::try_mutate(trading_pair, |(pool_0, pool_1)| -> Result<()> {
                let total_shares = totalIssuances::get(trading_pair.clone()).unwrap_or_default();
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
                                    .ok_or(Err(Error::ArithmeticOverflow))?,
                            )
                        } else {
                            (
                                ExchangeRate::checked_from_rational(max_amount_1, max_amount_0)
                                    .ok_or(Err(Error::ArithmeticOverflow))?,
                                ExchangeRate::one(),
                            )
                        };
    
                        let shares_from_token_0 = exchange_rate_0
                            .checked_mul_int(max_amount_0)
                            .ok_or(Err(Error::ArithmeticOverflow))?;
                        let shares_from_token_1 = exchange_rate_1
                            .checked_mul_int(max_amount_1)
                            .ok_or(Err(Error::ArithmeticOverflow))?;
                        let initial_shares = shares_from_token_0
                            .checked_add(shares_from_token_1)
                            .ok_or(Err(Error::ArithmeticOverflow))?;
    
                        (max_amount_0, max_amount_1, initial_shares)
                    } else {
                        let exchange_rate_0_1 =
                            ExchangeRate::checked_from_rational(*pool_1, *pool_0).ok_or(Err(Error::ArithmeticOverflow))?;
                        let input_exchange_rate_0_1 = ExchangeRate::checked_from_rational(max_amount_1, max_amount_0)
                            .ok_or(Err(Error::ArithmeticOverflow))?;
    
                        if input_exchange_rate_0_1 <= exchange_rate_0_1 {
                            // max_amount_0 may be too much, calculate the actual amount_0
                            let exchange_rate_1_0 =
                                ExchangeRate::checked_from_rational(*pool_0, *pool_1).ok_or(Err(Error::ArithmeticOverflow))?;
                            let amount_0 = exchange_rate_1_0
                                .checked_mul_int(max_amount_1)
                                .ok_or(Err(Error::ArithmeticOverflow))?;
                            let share_increment = Ratio::checked_from_rational(amount_0, *pool_0)
                                .and_then(|n| n.checked_mul_int(total_shares))
                                .ok_or(Err(Error::ArithmeticOverflow))?;
                            (amount_0, max_amount_1, share_increment)
                        } else {
                            // max_amount_1 is too much, calculate the actual amount_1
                            let amount_1 = exchange_rate_0_1
                                .checked_mul_int(max_amount_0)
                                .ok_or(Err(Error::ArithmeticOverflow))?;
                            let share_increment = Ratio::checked_from_rational(amount_1, *pool_1)
                                .and_then(|n| n.checked_mul_int(total_shares))
                                .ok_or(Err(Error::ArithmeticOverflow))?;
                            (max_amount_0, amount_1, share_increment)
                        }
                    };
    
                if share_increment.is_zero() || pool_0_increment.is_zero() || pool_1_increment.is_zero() {
                    return Err(Error::InvalidLiquidityIncrement)
                }

                if share_increment < min_share_increment {
                    return Err(Error::UnacceptableShareIncrement)
                }
    
                // Todo:
                // 1. Get uniswap account id
                // 2. transfer pool_0_increment amount of trading_pair.first() token from sender to uniswap account
                // 3. transfer pool_1_increment amount of trading_pair.second() token from sender to uniswap account
                // 4. totalIssuances[trading_pair].add(share_increment)
                // 5. share[trading_pair][who].add(share_increment)

                // let module_account_id = Self::account_id();
                // T::Currency::transfer(trading_pair.first(), who, &module_account_id, pool_0_increment)?;
                // T::Currency::transfer(trading_pair.second(), who, &module_account_id, pool_1_increment)?;
                // T::Currency::deposit(dex_share_currency_id, who, share_increment)?;
    
                *pool_0 = pool_0.checked_add(pool_0_increment).ok_or(Err(Error::ArithmeticOverflow))?;
                *pool_1 = pool_1.checked_add(pool_1_increment).ok_or(Err(Error::ArithmeticOverflow))?;
    
                self.env().emit_event(LiquidityAdded{
                    who.clone(),
                    trading_pair.first(),
                    pool_0_increment,
                    trading_pair.second(),
                    pool_1_increment,
                    share_increment,
                });
                Ok(())
            })
            Ok(())
        }
    
        fn do_remove_liquidity(
            &mut self,
            currency_id_a: TokenAddress,
            currency_id_b: TokenAddress,
            remove_share: Balance,
            min_withdrawn_a: Balance,
            min_withdrawn_b: Balance,
            by_unstake: bool,
        ) -> Result<()> {
            let caller = self.env().caller();

            if remove_share.is_zero() {
                return Ok(());
            }

            let trading_pair = TradingPair::from_currency_ids(currency_id_a, currency_id_b).ok_or(Err(Error::InvalidCurrencyId))?;
    
            LiquidityPool::<T>::try_mutate(trading_pair, |(pool_0, pool_1)| -> Result<()> {
                let (min_withdrawn_0, min_withdrawn_1) = if currency_id_a == trading_pair.first() {
                    (min_withdrawn_a, min_withdrawn_b)
                } else {
                    (min_withdrawn_b, min_withdrawn_a)
                };
                let total_shares = totalIssuances::get(trading_pair.clone()).unwrap_or_default();
                let proportion =
                    Ratio::checked_from_rational(remove_share, total_shares).ok_or(Err(Error::ArithmeticOverflow))?;
                let pool_0_decrement = proportion.checked_mul_int(*pool_0).ok_or(Err(Error::ArithmeticOverflow))?;
                let pool_1_decrement = proportion.checked_mul_int(*pool_1).ok_or(Err(Error::ArithmeticOverflow))?;

                if pool_0_decrement < min_withdrawn_0 || pool_1_decrement < min_withdrawn_1 {
                    return Err(Error::UnacceptableLiquidityWithdrawn)
                }

                // let module_account_id = Self::account_id();
    
                if by_unstake {
                    // T::DEXIncentives::do_withdraw_dex_share(who, dex_share_currency_id, remove_share)?;
                }

                // T::Currency::withdraw(dex_share_currency_id, &who, remove_share)?;
                // T::Currency::transfer(trading_pair.first(), &module_account_id, &who, pool_0_decrement)?;
                // T::Currency::transfer(trading_pair.second(), &module_account_id, &who, pool_1_decrement)?;
    
                *pool_0 = pool_0.checked_sub(pool_0_decrement).ok_or(Err(Error::ArithmeticOverflow))?;
                *pool_1 = pool_1.checked_sub(pool_1_decrement).ok_or(Err(Error::ArithmeticOverflow))?;
    
                self.env().emit_event(LiquidityRemoved{
                    who.clone(),
                    trading_pair.first(),
                    pool_0_decrement,
                    trading_pair.second(),
                    pool_1_decrement,
                    remove_share,
                });
                Ok(())
            })
        }
    
        /// 
        // fn get_liquidity(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
        //     if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b) {
        //         let (pool_0, pool_1) = Self::liquidity_pool(trading_pair);
        //         if currency_id_a == trading_pair.first() {
        //             (pool_0, pool_1)
        //         } else {
        //             (pool_1, pool_0)
        //         }
        //     } else {
        //         (Zero::zero(), Zero::zero())
        //     }
        // }
    

        // Simply returns the current value of our `bool`.
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
