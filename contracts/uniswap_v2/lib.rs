#![cfg_attr(not(feature = "std"), no_std)]

mod chain_extension;
mod constants;
mod errors;
mod models;
mod mock;

use errors::Error;
use ink_lang as ink;

pub type Result<T> = core::result::Result<T, Error>;

#[ink::contract(env = crate::chain_extension::CustomEnvironment)]
mod uniswap_v2 {
    use super::*;
    use crate::{
        constants::{GET_EXCHANGE_FEE, TRADING_PATH_LIMIT},
        models::{ExchangeRate, Ratio, AssetId, TradingPair},
    };
    use core::convert::TryInto;
    use ink_prelude::vec;
    use ink_prelude::vec::Vec;
    use ink_storage::collections::HashMap;
    use num_traits::Zero;
    use primitive_types::U256;

    #[ink(storage)]
    pub struct UniswapV2 {
        /// Deployer account
        owner: AccountId,
        /// Stores the balance of each token for a trading pair
        liquidity_pool: HashMap<TradingPair, (Balance, Balance)>,
        /// Total LP amount for a trading pair
        total_issuances: HashMap<TradingPair, Balance>,
        /// LP amount for a specific user
        dex_incentives: HashMap<(TradingPair, AccountId), Balance>,
    }

    /// Emitted when Adding liquidity success. \[who, currency_id_0, pool_0_increment, currency_id_1, pool_1_increment, share_increment\].
    #[ink(event)]
    pub struct LiquidityAdded {
        who: AccountId,
        currency_id_0: AssetId,
        pool_0_increment: Balance,
        currency_id_1: AssetId,
        pool_1_increment: Balance,
        share_increment: Balance,
    }

    /// Emitted when Removing liquidity from the trading pool success. \[who, currency_id_0, pool_0_decrement, currency_id_1, pool_1_decrement, share_decrement\]
    #[ink(event)]
    pub struct LiquidityRemoved {
        who: AccountId,
        currency_id_0: AssetId,
        pool_0_decrement: Balance,
        currency_id_1: AssetId,
        pool_1_decrement: Balance,
        remove_share: Balance,
    }

    /// Use supply currency to swap target currency. \[trader, trading_path, supply_currency_amount, target_currency_amount\]
    #[ink(event)]
    pub struct Swap {
        who: AccountId,
        path: Vec<AssetId>,
        supply: Balance,
        target: Balance,
    }

    impl UniswapV2 {
        /// Constructor
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                owner: Self::env().caller(),
                liquidity_pool: HashMap::new(),
                total_issuances: HashMap::new(),
                dex_incentives: HashMap::new(),
            }
        }

        /// Add liquidity to trading pair
        /// - `currency_id_a`: currency id A.
        /// - `currency_id_b`: currency id B.
        /// - `max_amount_a`: maximum amount of currency_id_a is allowed to inject to liquidity pool.
        /// - `max_amount_b`: maximum amount of currency_id_b is allowed to inject to liquidity pool.
        /// - `min_share_increment`: minimum acceptable share amount.
        /// - `stake_increment_share`: indicates whether to stake increased dex share to earn incentives
        #[ink(message)]
        pub fn add_liquidity(
            &mut self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
            max_amount_a: Balance,
            max_amount_b: Balance,
            min_share_increment: Balance,
            stake_increment_share: bool,
        ) -> Result<()> {
            self.do_add_liquidity(
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
            &mut self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
            remove_share: Balance,
            min_withdrawn_a: Balance,
            min_withdrawn_b: Balance,
            by_unstake: bool,
        ) -> Result<()> {
            self.do_remove_liquidity(
                currency_id_a,
                currency_id_b,
                remove_share,
                min_withdrawn_a,
                min_withdrawn_b,
                by_unstake,
            )?;
            Ok(())
        }

        /// Swap with exact supply amount
        ///
        /// - `path`: trading path.
        /// - `supply_amount`: exact supply amount.
        /// - `min_target_amount`: acceptable minimum target amount.
        #[ink(message)]
        pub fn swap_with_exact_supply(
            &mut self,
            path: Vec<AssetId>,
            supply_amount: Balance,
            min_target_amount: Balance,
        ) -> Result<()> {
            self.do_swap_with_exact_supply(path, supply_amount, min_target_amount, None)?;
            Ok(())
        }

        /// Swap with exact target amount
        ///
        /// - `path`: trading path.
        /// - `target_amount`: exact target amount.
        /// - `max_supply_amount`: acceptable maximum supply amount.
        #[ink(message)]
        pub fn swap_with_exact_target(
            &mut self,
            path: Vec<AssetId>,
            target_amount: Balance,
            max_supply_amount: Balance,
        ) -> Result<()> {
            self.do_swap_with_exact_target(path, target_amount, max_supply_amount, None)?;
            Ok(())
        }

        pub fn get_target_amount(
            &self,
            supply_pool: Balance,
            target_pool: Balance,
            supply_amount: Balance,
        ) -> Balance {
            if supply_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
                Zero::zero()
            } else {
                let (fee_numerator, fee_denominator) = GET_EXCHANGE_FEE;
                let supply_amount_with_fee: U256 = U256::from(supply_amount)
                    .saturating_mul(U256::from(fee_denominator.saturating_sub(fee_numerator)));
                let numerator: U256 =
                    supply_amount_with_fee.saturating_mul(U256::from(target_pool));
                let denominator: U256 = U256::from(supply_pool)
                    .saturating_mul(U256::from(fee_denominator))
                    .saturating_add(supply_amount_with_fee);

                numerator
                    .checked_div(denominator)
                    .and_then(|n| TryInto::<Balance>::try_into(n).ok())
                    .unwrap_or_else(Zero::zero)
            }
        }

        pub fn get_supply_amount(
            &self,
            supply_pool: Balance,
            target_pool: Balance,
            target_amount: Balance,
        ) -> Balance {
            if target_amount.is_zero() || supply_pool.is_zero() || target_pool.is_zero() {
                Zero::zero()
            } else {
                let (fee_numerator, fee_denominator) = GET_EXCHANGE_FEE;
                let numerator: U256 = U256::from(supply_pool)
                    .saturating_mul(U256::from(target_amount))
                    .saturating_mul(U256::from(fee_denominator));
                let denominator: U256 = U256::from(target_pool)
                    .saturating_sub(U256::from(target_amount))
                    .saturating_mul(U256::from(fee_denominator.saturating_sub(fee_numerator)));

                numerator
                    .checked_div(denominator)
                    .and_then(|r| r.checked_add(U256::one())) // add 1 to result so that correct the possible losses caused by remainder discarding in
                    .and_then(|n| TryInto::<Balance>::try_into(n).ok())
                    .unwrap_or_else(Zero::zero)
            }
        }

        pub fn get_target_amounts(
            &self,
            path: &Vec<AssetId>,
            supply_amount: Balance,
            price_impact_limit: Option<Ratio>,
        ) -> Result<Vec<Balance>> {
            let path_length = path.len();
            if path_length < 2 || path_length > (TRADING_PATH_LIMIT as usize) {
                return Err(Error::InvalidTradingPathLength);
            }

            let mut target_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
            target_amounts[0] = supply_amount;

            let mut i: usize = 0;
            while i + 1 < path_length {
                let (supply_pool, target_pool) = self.get_liquidity(path[i], path[i + 1]);
                if supply_pool.is_zero() || target_pool.is_zero() {
                    return Err(Error::InsufficientLiquidity);
                }
                let target_amount =
                    self.get_target_amount(supply_pool, target_pool, target_amounts[i]);

                if target_amount.is_zero() {
                    return Err(Error::ZeroTargetAmount);
                }

                // check price impact if limit exists
                if let Some(limit) = price_impact_limit {
                    let price_impact = Ratio::from_num(target_amount)
                        .checked_div_int(target_pool)
                        .unwrap_or_else(Ratio::max_value);
                    if price_impact > limit {
                        return Err(Error::ExceedPriceImpactLimit);
                    }
                }

                target_amounts[i + 1] = target_amount;
                i += 1;
            }

            Ok(target_amounts)
        }

        pub fn get_supply_amounts(
            &self,
            path: &Vec<AssetId>,
            target_amount: Balance,
            price_impact_limit: Option<Ratio>,
        ) -> Result<Vec<Balance>> {
            let path_length = path.len();
            if path_length < 2 || path_length > (TRADING_PATH_LIMIT as usize) {
                return Err(Error::InvalidTradingPathLength);
            }

            let mut supply_amounts: Vec<Balance> = vec![Zero::zero(); path_length];
            supply_amounts[path_length - 1] = target_amount;

            let mut i: usize = path_length - 1;
            while i > 0 {
                let (supply_pool, target_pool) = self.get_liquidity(path[i - 1], path[i]);
                if supply_pool.is_zero() || target_pool.is_zero() {
                    return Err(Error::InsufficientLiquidity);
                }
                let supply_amount =
                    self.get_supply_amount(supply_pool, target_pool, supply_amounts[i]);

                if supply_amount.is_zero() {
                    return Err(Error::ZeroSupplyAmount);
                }

                // check price impact if limit exists
                if let Some(limit) = price_impact_limit {
                    let price_impact = Ratio::from_num(supply_amounts[i])
                        .checked_div_int(target_pool)
                        .unwrap_or_else(Ratio::max_value);
                    if price_impact > limit {
                        return Err(Error::ExceedPriceImpactLimit);
                    }
                }

                supply_amounts[i - 1] = supply_amount;
                i -= 1;
            }

            Ok(supply_amounts)
        }

        fn _swap(
            &mut self,
            supply_currency_id: AssetId,
            target_currency_id: AssetId,
            supply_increment: Balance,
            target_decrement: Balance,
        ) -> Result<()> {
            if let Some(trading_pair) =
                TradingPair::from_currency_ids(supply_currency_id, target_currency_id)
            {
                let ((pool_0, pool_1), _) = self.pair_info(&trading_pair);

                let invariant_before_swap: U256 =
                    U256::from(pool_0).saturating_mul(U256::from(pool_1));

                let pool_0_after;
                let pool_1_after;

                if supply_currency_id == trading_pair.first() {
                    pool_0_after = pool_0
                        .checked_add(supply_increment)
                        .ok_or(Error::ArithmeticOverflow)?;
                    pool_1_after = pool_1
                        .checked_sub(target_decrement)
                        .ok_or(Error::ArithmeticUnderflow)?;
                } else {
                    pool_0_after = pool_0
                        .checked_sub(target_decrement)
                        .ok_or(Error::ArithmeticUnderflow)?;
                        pool_1_after = pool_1
                        .checked_add(supply_increment)
                        .ok_or(Error::ArithmeticOverflow)?;
                }

                // invariant check to ensure the constant product formulas (k = x * y)
                let invariant_after_swap: U256 =
                    U256::from(pool_0_after).saturating_mul(U256::from(pool_1_after));

                if invariant_after_swap < invariant_before_swap {
                    return Err(Error::InvariantCheckFailed);
                }

                self.liquidity_pool.insert(trading_pair.clone(), (pool_0_after, pool_1_after));

                return Ok(());
            }
            Err(Error::InvalidTradingPair)
        }

        fn _swap_by_path(&mut self, path: &[AssetId], amounts: &[Balance]) -> Result<()> {
            let mut i: usize = 0;
            if path.len() != amounts.len() {
                return Err(Error::InvalidPathAmountsLength);
            }
            while i + 1 < path.len() {
                let (supply_currency_id, target_currency_id) = (path[i], path[i + 1]);
                let (supply_increment, target_decrement) = (amounts[i], amounts[i + 1]);
                self._swap(
                    supply_currency_id,
                    target_currency_id,
                    supply_increment,
                    target_decrement,
                )?;
                i += 1;
            }
            Ok(())
        }

        pub fn do_swap_with_exact_supply(
            &mut self,
            path: Vec<AssetId>,
            supply_amount: Balance,
            min_target_amount: Balance,
            price_impact_limit: Option<Ratio>,
        ) -> Result<Balance> {
            let caller = self.env().caller();

            let amounts = self.get_target_amounts(&path, supply_amount, price_impact_limit)?;

            if amounts.len() < 1 {
                return Err(Error::InvalidAmountsLength);
            }

            if amounts[amounts.len() - 1] < min_target_amount {
                return Err(Error::InsufficientTargetAmount);
            }

            let actual_target_amount = amounts[amounts.len() - 1];

            self.env()
                .extension()
                .deposit(path[0], caller, supply_amount)?;
            self._swap_by_path(&path, &amounts)?;
            self.env()
                .extension()
                .withdraw(path[path.len() - 1], caller, actual_target_amount)?;

            self.env().emit_event(Swap {
                who: caller,
                path: path.to_vec(),
                supply: supply_amount,
                target: actual_target_amount,
            });

            Ok(actual_target_amount)
        }

        pub fn do_swap_with_exact_target(
            &mut self,
            path: Vec<AssetId>,
            target_amount: Balance,
            max_supply_amount: Balance,
            price_impact_limit: Option<Ratio>,
        ) -> Result<Balance> {
            let caller = self.env().caller();

            let amounts = self.get_supply_amounts(&path, target_amount, price_impact_limit)?;

            if amounts.len() < 1 {
                return Err(Error::InvalidAmountsLength);
            }

            if amounts[0] > max_supply_amount {
                return Err(Error::ExcessiveSupplyAmount);
            }

            let actual_supply_amount = amounts[0];

            self.env()
                .extension()
                .deposit(path[0], caller, actual_supply_amount)?;
            self._swap_by_path(&path, &amounts)?;
            self.env()
                .extension()
                .withdraw(path[path.len() - 1], caller, target_amount)?;

            self.env().emit_event(Swap {
                who: caller,
                path: path.to_vec(),
                supply: actual_supply_amount,
                target: target_amount,
            });

            Ok(actual_supply_amount)
        }

        fn pair_info(&mut self, trading_pair: &TradingPair) -> ((Balance, Balance), Balance) {
            let (pool_0, pool_1): (Balance, Balance) = match self.liquidity_pool.get(trading_pair) {
                Some((p_0, p_1)) => (*p_0, *p_1),
                None => (Zero::zero(), Zero::zero()),
            };

            let total_shares = match self.total_issuances.get(trading_pair) {
                Some(share) => *share,
                None => Zero::zero(),
            };

            ((pool_0, pool_1), total_shares)
        }

        fn do_deposit_dex_share(
            &mut self,
            who: AccountId,
            trading_pair: &TradingPair,
            share_increment: Balance,
        ) -> Result<()> {
            let incentives = match self.dex_incentives.get(&(trading_pair.clone(), who)) {
                Some(p) => *p,
                None => Zero::zero(),
            };

            let incentives = incentives
                .checked_add(share_increment)
                .ok_or(Error::ArithmeticOverflow)?;

            self.dex_incentives
                .insert((trading_pair.clone(), who), incentives);

            let (_, total_shares) = self.pair_info(&trading_pair);

            let total_shares = total_shares
                .checked_add(share_increment)
                .ok_or(Error::ArithmeticOverflow)?;

            self.total_issuances
                .insert(trading_pair.clone(), total_shares);

            Ok(())
        }

        fn do_withdraw_dex_share(
            &mut self,
            who: AccountId,
            trading_pair: &TradingPair,
            share_decrement: Balance,
        ) -> Result<()> {
            let incentives = match self.dex_incentives.get(&(trading_pair.clone(), who)) {
                Some(p) => *p,
                None => Zero::zero(),
            };

            let incentives = incentives
                .checked_sub(share_decrement)
                .ok_or(Error::ArithmeticOverflow)?;

            self.dex_incentives
                .insert((trading_pair.clone(), who), incentives);

            let (_, total_shares) = self.pair_info(&trading_pair);

            let total_shares = total_shares
                .checked_sub(share_decrement)
                .ok_or(Error::ArithmeticOverflow)?;

            self.total_issuances
                .insert(trading_pair.clone(), total_shares);

            Ok(())
        }

        fn do_deposit_pool(
            &mut self,
            trading_pair: &TradingPair,
            pool_0_increment: Balance,
            pool_1_increment: Balance,
        ) -> Result<(Balance, Balance)> {
            let ((pool_0, pool_1), _) = self.pair_info(&trading_pair);
            let pool_0 = pool_0
                .checked_add(pool_0_increment)
                .ok_or(Error::ArithmeticOverflow)?;
            let pool_1 = pool_1
                .checked_add(pool_1_increment)
                .ok_or(Error::ArithmeticOverflow)?;

            self.liquidity_pool
                .insert(trading_pair.clone(), (pool_0, pool_1));
            Ok((pool_0, pool_1))
        }

        fn do_withdraw_pool(
            &mut self,
            trading_pair: &TradingPair,
            pool_0_decrement: Balance,
            pool_1_decrement: Balance,
        ) -> Result<(Balance, Balance)> {
            let ((pool_0, pool_1), _) = self.pair_info(&trading_pair);
            let pool_0 = pool_0
                .checked_sub(pool_0_decrement)
                .ok_or(Error::ArithmeticOverflow)?;
            let pool_1 = pool_1
                .checked_sub(pool_1_decrement)
                .ok_or(Error::ArithmeticOverflow)?;

            self.liquidity_pool
                .insert(trading_pair.clone(), (pool_0, pool_1));
            Ok((pool_0, pool_1))
        }

        fn do_add_liquidity(
            &mut self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
            max_amount_a: Balance,
            max_amount_b: Balance,
            min_share_increment: Balance,
            _stake_increment_share: bool,
        ) -> Result<()> {
            let caller = self.env().caller();

            let trading_pair = TradingPair::from_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::InvalidAssetId)?;

            if max_amount_a.is_zero() || max_amount_b.is_zero() {
                return Err(Error::InvalidLiquidityIncrement);
            }

            let ((pool_0, pool_1), total_shares) = self.pair_info(&trading_pair);

            let (max_amount_0, max_amount_1) = if currency_id_a == trading_pair.first() {
                (max_amount_a, max_amount_b)
            } else {
                (max_amount_b, max_amount_a)
            };

            let (pool_0_increment, pool_1_increment, share_increment): (Balance, Balance, Balance) =
                if total_shares.is_zero() {
                    let exchange_rate_0 = ExchangeRate::from_num(1);
                    let exchange_rate_1 = ExchangeRate::from_num(max_amount_0)
                        .checked_div_int(max_amount_1)
                        .ok_or(Error::ArithmeticOverflow)?;

                    let shares_from_token_0 = exchange_rate_0
                        .checked_mul_int(max_amount_0)
                        .ok_or(Error::ArithmeticOverflow)?;
                    let shares_from_token_1 = exchange_rate_1
                        .checked_mul_int(max_amount_1)
                        .ok_or(Error::ArithmeticOverflow)?;
                    let initial_shares = shares_from_token_0
                        .checked_add(shares_from_token_1)
                        .ok_or(Error::ArithmeticOverflow)?;

                    (max_amount_0, max_amount_1, initial_shares.to_num())
                } else {
                    let exchange_rate_0_1 = ExchangeRate::from_num(pool_1)
                        .checked_div_int(pool_0)
                        .ok_or(Error::ArithmeticOverflow)?;

                    let input_exchange_rate_0_1 = ExchangeRate::from_num(max_amount_1)
                        .checked_div_int(max_amount_0)
                        .ok_or(Error::ArithmeticOverflow)?;

                    if input_exchange_rate_0_1 <= exchange_rate_0_1 {
                        // max_amount_0 may be too much, calculate the actual amount_0
                        let exchange_rate_1_0 = ExchangeRate::from_num(pool_0)
                            .checked_div_int(pool_1)
                            .ok_or(Error::ArithmeticOverflow)?;

                        let amount_0 = exchange_rate_1_0
                            .checked_mul_int(max_amount_1)
                            .ok_or(Error::ArithmeticOverflow)?;

                        let share_increment = ExchangeRate::from_num(amount_0)
                            .checked_div_int(pool_0)
                            .ok_or(Error::ArithmeticOverflow)?;

                        let share_increment = share_increment
                            .checked_mul_int(total_shares)
                            .ok_or(Error::ArithmeticOverflow)?;

                        (amount_0.to_num(), max_amount_1, share_increment.to_num())
                    } else {
                        // max_amount_1 is too much, calculate the actual amount_1
                        let amount_1 = exchange_rate_0_1
                            .checked_mul_int(max_amount_0)
                            .ok_or(Error::ArithmeticOverflow)?;

                        let share_increment = ExchangeRate::from_num(amount_1)
                            .checked_div_int(pool_1)
                            .ok_or(Error::ArithmeticOverflow)?;

                        let share_increment = share_increment
                            .checked_mul_int(total_shares)
                            .ok_or(Error::ArithmeticOverflow)?;
                        (max_amount_0, amount_1.to_num(), share_increment.to_num())
                    }
                };

            if share_increment.is_zero() || pool_0_increment.is_zero() || pool_1_increment.is_zero()
            {
                return Err(Error::InvalidLiquidityIncrement);
            }

            if share_increment < min_share_increment {
                return Err(Error::UnacceptableShareIncrement);
            }

            self.env()
                .extension()
                .deposit(trading_pair.first(), caller, pool_0_increment)?;
            self.env()
                .extension()
                .deposit(trading_pair.second(), caller, pool_1_increment)?;

            self.do_deposit_pool(&trading_pair, pool_0_increment, pool_1_increment)?;
            self.do_deposit_dex_share(caller, &trading_pair, share_increment)?;

            self.env().emit_event(LiquidityAdded {
                who: caller,
                currency_id_0: trading_pair.first(),
                pool_0_increment,
                currency_id_1: trading_pair.second(),
                pool_1_increment,
                share_increment,
            });

            Ok(())
        }

        fn do_remove_liquidity(
            &mut self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
            remove_share: Balance,
            min_withdrawn_a: Balance,
            min_withdrawn_b: Balance,
            _by_unstake: bool,
        ) -> Result<()> {
            let caller = self.env().caller();

            if remove_share.is_zero() {
                return Ok(());
            }

            let trading_pair = TradingPair::from_currency_ids(currency_id_a, currency_id_b)
                .ok_or(Error::InvalidAssetId)?;

            let ((pool_0, pool_1), total_shares) = self.pair_info(&trading_pair);

            let (min_withdrawn_0, min_withdrawn_1) = if currency_id_a == trading_pair.first() {
                (min_withdrawn_a, min_withdrawn_b)
            } else {
                (min_withdrawn_b, min_withdrawn_a)
            };

            let proportion = ExchangeRate::from_num(remove_share)
                .checked_div_int(total_shares)
                .ok_or(Error::ArithmeticOverflow)?;

            let pool_0_decrement = proportion
                .checked_mul_int(pool_0)
                .ok_or(Error::ArithmeticOverflow)?
                .to_num();
            let pool_1_decrement = proportion
                .checked_mul_int(pool_1)
                .ok_or(Error::ArithmeticOverflow)?
                .to_num();

            if pool_0_decrement < min_withdrawn_0 || pool_1_decrement < min_withdrawn_1 {
                return Err(Error::UnacceptableLiquidityWithdrawn);
            }

            self.env()
                .extension()
                .withdraw(trading_pair.first(), caller, pool_0_decrement)?;

            self.env()
                .extension()
                .withdraw(trading_pair.second(), caller, pool_1_decrement)?;

            self.do_withdraw_pool(&trading_pair, pool_0_decrement, pool_1_decrement)?;
            self.do_withdraw_dex_share(caller, &trading_pair, remove_share)?;

            self.env().emit_event(LiquidityRemoved {
                who: caller,
                currency_id_0: trading_pair.first(),
                pool_0_decrement,
                currency_id_1: trading_pair.second(),
                pool_1_decrement,
                remove_share,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn get_liquidity(
            &self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
        ) -> (Balance, Balance) {
            if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b)
            {
                let (pool_0, pool_1): (Balance, Balance) =
                    match self.liquidity_pool.get(&trading_pair) {
                        Some((p_0, p_1)) => (*p_0, *p_1),
                        None => (Zero::zero(), Zero::zero()),
                    };
                if currency_id_a == trading_pair.first() {
                    (pool_0, pool_1)
                } else {
                    (pool_1, pool_0)
                }
            } else {
                (Zero::zero(), Zero::zero())
            }
        }

        #[ink(message)]
        pub fn get_dex_incentive(
            &self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
            account: AccountId,
        ) -> Balance {
            if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b)
            {
                match self.dex_incentives.get(&(trading_pair, account)) {
                    Some(p) => *p,
                    None => Zero::zero(),
                }
            } else {
                Zero::zero()
            }
        }

        #[ink(message)]
        pub fn get_total_issuance(
            &self,
            currency_id_a: AssetId,
            currency_id_b: AssetId,
        ) -> Balance {
            if let Some(trading_pair) = TradingPair::from_currency_ids(currency_id_a, currency_id_b)
            {
                match self.total_issuances.get(&trading_pair) {
                    Some(p) => *p,
                    None => Zero::zero(),
                }
            } else {
                Zero::zero()
            }
        }

        #[ink(message)]
        pub fn get_swap_target_amount(
            &self,
            path: Vec<AssetId>,
            supply_amount: Balance,
            // price_impact_limit: Option<Ratio>,
        ) -> Option<Balance> {
            let path_length = path.len();
            if path_length < 2 || path_length > (TRADING_PATH_LIMIT as usize) {
                return Some(1);
            }

            self.get_target_amounts(&path, supply_amount, None)
                .ok()
                .map(|amounts| amounts[amounts.len() - 1])
        }

        #[ink(message)]
        pub fn get_swap_supply_amount(
            &self,
            path: Vec<AssetId>,
            target_amount: Balance,
            // price_impact_limit: Option<Ratio>,
        ) -> Option<Balance> {
            let path_length = path.len();
            if path_length < 2 || path_length > (TRADING_PATH_LIMIT as usize) {
                return Some(1);
            }

            self.get_supply_amounts(&path, target_amount, None)
                .ok()
                .map(|amounts| amounts[0])
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(feature = "ink-experimental-engine")]
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{
            mock::{PDEX, BTC, DOT}
        };
        type Event = <UniswapV2 as ink_lang::reflect::ContractEventBase>::Type;
        
        use ink_lang as ink;

        #[ink::test]
        fn add_liquidity_works() {
            let mut uniswap = UniswapV2::new();
            let amount_a = 100000;
            let amount_b = 20000;

            let liquidity = uniswap.get_liquidity(PDEX, DOT);
            assert_eq!(liquidity, (0, 0));

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let liquidity = uniswap.get_liquidity(PDEX, DOT);
            assert_eq!(liquidity, (amount_a, amount_b));

            let liquidity = uniswap.get_liquidity(DOT, PDEX);
            assert_eq!(liquidity, (amount_b, amount_a));

            let total_issuance = uniswap.get_total_issuance(PDEX, DOT);
            assert_eq!(total_issuance, 200000);

            let amount_c = 20000;
            let amount_d = 40000;
            let result = uniswap.add_liquidity(PDEX, DOT, amount_c, amount_d, 0, true);
            assert_eq!(result, Ok(()));

            let liquidity = uniswap.get_liquidity(PDEX, DOT);
            assert_eq!(liquidity, (amount_a + 20_000, amount_b + 3_999));

            let total_issuance = uniswap.get_total_issuance(PDEX, DOT);
            // env::debug_println!("{:?}", total_issuance);
            assert_eq!(total_issuance, 239_999);
        }

        #[ink::test]
        fn add_liquidity_works_with_big_number() {
            let swap_test_cases = [
                (
                    1_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    5_000_000_000_000_000_000,
                    453_305_446_940_074_565
                ),
                (
                    1_000_000_000_000_000_000,
                    5_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    1_662_497_915_624_478_906
                ),
                (
                    2_000_000_000_000_000_000,
                    5_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    2_851_015_155_847_869_602
                ),
                (
                    2_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    5_000_000_000_000_000_000,
                    831_248_957_812_239_453
                ),
                (
                    1_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    10_000_000_000_000_000_000,
                    906_610_893_880_149_131
                ),
                (
                    1_000_000_000_000_000_000,
                    100_000_000_000_000_000_000,
                    100_000_000_000_000_000_000,
                    987_158_034_397_061_298
                ),
                (
                    1_000_000_000_000_000_000,
                    1_000_000_000_000_000_000_000,
                    1_000_000_000_000_000_000_000,
                    996_006_981_039_903_216
                ),
            ];

            for i in 0..swap_test_cases.len() {
                let mut uniswap = UniswapV2::new();
                let (swap_amount, amount_a, amount_b, expected_output_amount) = swap_test_cases[i];
                let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
                assert_eq!(result, Ok(()));

                let test_target_amount = uniswap.get_swap_target_amount([PDEX, DOT].to_vec(), swap_amount);
                assert_eq!(test_target_amount, Some(expected_output_amount));
            }
        }

        #[ink::test]
        fn remove_liquidity_works() {
            let mut uniswap = UniswapV2::new();
            let BOB = AccountId::from([0x1; 32]);
            let ALICE = AccountId::from([0x2; 32]);
            let amount_a = 100000;
            let amount_b = 20000;
            let total_share = 200000;
            let remove_share = 50000;
            let decrement_a = 25000;
            let decrement_b = 5000;

            let liquidity = uniswap.get_liquidity(PDEX, DOT);
            assert_eq!(liquidity, (0, 0));

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(BOB);

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let result = uniswap.remove_liquidity(PDEX, DOT, remove_share, 0, 0, true);
            assert_eq!(result, Ok(()));

            let liquidity = uniswap.get_liquidity(PDEX, DOT);
            assert_eq!(liquidity, (amount_a - decrement_a, amount_b - decrement_b));

            let total_issuance = uniswap.get_total_issuance(PDEX, DOT);
            assert_eq!(total_issuance, total_share - remove_share);

            let dex_incentive_bob = uniswap.get_dex_incentive(PDEX, DOT, BOB);
            assert_eq!(dex_incentive_bob, total_share - remove_share);

            let dex_incentive_alice = uniswap.get_dex_incentive(PDEX, DOT, ALICE);
            assert_eq!(dex_incentive_alice, 0);
        }

        #[ink::test]
        fn single_path_swap_works() {
            let mut uniswap = UniswapV2::new();
            let amount_a = 100_000_000_000_000_000_000;
            let amount_b = 20_000_000_000_000_000_000;

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let test_target_amount = uniswap.get_swap_target_amount([PDEX, DOT].to_vec(), 20_000_000_000_000_000_000);
            assert_eq!(test_target_amount, Some(3_324_995_831_248_957_812));
            
            let test_swap_amount = uniswap.get_swap_supply_amount([PDEX, DOT].to_vec(), 3_324_995_831_248_957_812);
            assert_eq!(test_swap_amount, Some(19_999_999_999_999_999_999));

            let result = uniswap.swap_with_exact_supply([PDEX, DOT].to_vec(), 20_000_000_000_000_000_000, 0);
            assert_eq!(result, Ok(()));

            let result = uniswap.swap_with_exact_target([DOT, PDEX].to_vec(), 10_000_000_000_000_000_000, 2_000_000_000_000_000_000);
            assert_eq!(result, Ok(()));
        }

        #[ink::test]
        fn swap_with_exact_supply_fails_with_insufficient_target_amount() {
            let mut uniswap = UniswapV2::new();
            let amount_a = 100_000_000_000_000_000_000;
            let amount_b = 20_000_000_000_000_000_000;

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let result = uniswap.swap_with_exact_supply([PDEX, DOT].to_vec(), 20_000_000_000_000_000_000, 20_000_000_000_000_000_000);
            assert_eq!(result, Err(Error::InsufficientTargetAmount));
        }

        #[ink::test]
        fn swap_with_exact_supply_works() {
            let mut uniswap = UniswapV2::new();
            let ALICE = AccountId::from([0x1; 32]);
            let amount_a = 100_000_000_000_000_000_000;
            let amount_b = 20_000_000_000_000_000_000;
            let supply_amount = 20_000_000_000_000_000_000;

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(ALICE);

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let result = uniswap.swap_with_exact_supply([PDEX, DOT].to_vec(), supply_amount, 0);
            assert_eq!(result, Ok(()));

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 2);

            let decoded_event = <Event as scale::Decode>::decode(&mut &emitted_events[1].data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::Swap(Swap { who, path, supply, target }) = decoded_event {
                assert_eq!(who, ALICE, "encountered invalid Swap.who");
                assert_eq!(path, [PDEX, DOT].to_vec(), "encountered invalid Swap.path");
                assert_eq!(supply, supply_amount, "encountered invalid Swap.supply");
                assert_eq!(target, 3_324_995_831_248_957_812, "encountered invalid Swap.target");
            } else {
                panic!("encountered unexpected event kind: expected a Swap event")
            }
        }

        #[ink::test]
        fn swap_with_exact_target_fails_with_excessive_supply_amount() {
            let mut uniswap = UniswapV2::new();
            let amount_a = 100_000_000_000_000_000_000;
            let amount_b = 20_000_000_000_000_000_000;

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let result = uniswap.swap_with_exact_target([DOT, PDEX].to_vec(), 10_000_000_000_000_000_000, 0);
            assert_eq!(result, Err(Error::ExcessiveSupplyAmount));
        }

        #[ink::test]
        fn swap_with_exact_target_works() {
            let mut uniswap = UniswapV2::new();
            let ALICE = AccountId::from([0x1; 32]);
            let amount_a = 100_000_000_000_000_000_000;
            let amount_b = 20_000_000_000_000_000_000;
            let target_amount = 10_000_000_000_000_000_000;

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(ALICE);

            let result = uniswap.add_liquidity(PDEX, DOT, amount_a, amount_b, 0, true);
            assert_eq!(result, Ok(()));

            let result = uniswap.swap_with_exact_target([DOT, PDEX].to_vec(), target_amount, 3_000_000_000_000_000_000);
            assert_eq!(result, Ok(()));

            let emitted_events = ink_env::test::recorded_events().collect::<Vec<_>>();
            assert_eq!(emitted_events.len(), 2);

            let decoded_event = <Event as scale::Decode>::decode(&mut &emitted_events[1].data[..])
                .expect("encountered invalid contract event data buffer");

            if let Event::Swap(Swap { who, path, supply, target }) = decoded_event {
                assert_eq!(who, ALICE, "encountered invalid Swap.who");
                assert_eq!(path, [DOT, PDEX].to_vec(), "encountered invalid Swap.path");
                assert_eq!(supply, 2_228_908_949_069_430_514, "encountered invalid Swap.supply");
                assert_eq!(target, target_amount, "encountered invalid Swap.target");
            } else {
                panic!("encountered unexpected event kind: expected a Swap event")
            }
        }

        #[ink::test]
        fn get_target_amount_works() {
            let uniswap = UniswapV2::new();

            assert_eq!(uniswap.get_target_amount(10000, 0, 1000), 0);
            assert_eq!(uniswap.get_target_amount(0, 20000, 1000), 0);
            assert_eq!(uniswap.get_target_amount(10000, 20000, 0), 0);
            assert_eq!(uniswap.get_target_amount(10000, 1, 1000000), 0);
            assert_eq!(uniswap.get_target_amount(10000, 20000, 10000), 9984);
            assert_eq!(uniswap.get_target_amount(10000, 20000, 1000), 1813);
        }

        #[ink::test]
        fn get_supply_amount_works() {
            let uniswap = UniswapV2::new();

            assert_eq!(uniswap.get_supply_amount(10000, 0, 1000), 0);
            assert_eq!(uniswap.get_supply_amount(0, 20000, 1000), 0);
            assert_eq!(uniswap.get_supply_amount(10000, 20000, 0), 0);
            assert_eq!(uniswap.get_supply_amount(10000, 1, 1), 0);
            assert_eq!(uniswap.get_supply_amount(10000, 20000, 9949), 9929);
            assert_eq!(uniswap.get_supply_amount(10000, 20000, 1801), 993);
        }

        #[ink::test]
        fn get_target_amounts_works() {
            let mut uniswap = UniswapV2::new();

            assert_eq!(
                uniswap.add_liquidity(PDEX, DOT, 50000, 10000, 0, true),
                Ok(())    
            );
            assert_eq!(
                uniswap.add_liquidity(PDEX, BTC, 100000, 10, 0, true),
                Ok(())
            );

            assert_eq!(
				uniswap.get_target_amounts(&[DOT].to_vec(), 10000, None),
				Err(Error::InvalidTradingPathLength),
			);
            assert_eq!(
				uniswap.get_target_amounts(&[DOT, PDEX, BTC, DOT].to_vec(), 10000, None),
				Err(Error::InvalidTradingPathLength),
			);
            assert_eq!(
				uniswap.get_target_amounts(&[DOT, PDEX].to_vec(), 10000, None),
				Ok(vec![10000, 24962])
			);
            assert_eq!(
				uniswap.get_target_amounts(&[DOT, PDEX, BTC].to_vec(), 10000, None),
				Ok(vec![10000, 24962, 1])
			);
            assert_eq!(
				uniswap.get_target_amounts(&[DOT, PDEX, BTC].to_vec(), 100, None),
				Err(Error::ZeroTargetAmount),
			);
			assert_eq!(
				uniswap.get_target_amounts(&[DOT, BTC].to_vec(), 100, None),
				Err(Error::InsufficientLiquidity),
			);
        }

        #[ink::test]
        fn get_supply_amounts_works() {
            let mut uniswap = UniswapV2::new();

            assert_eq!(
                uniswap.add_liquidity(PDEX, DOT, 50000, 10000, 0, true),
                Ok(())
            );
            assert_eq!(
                uniswap.add_liquidity(PDEX, BTC, 100000, 10, 0, true),
                Ok(())
            );

            assert_eq!(
				uniswap.get_supply_amounts(&[DOT].to_vec(), 10000, None),
				Err(Error::InvalidTradingPathLength),
			);
            assert_eq!(
				uniswap.get_supply_amounts(&[DOT, PDEX, BTC, DOT].to_vec(), 10000, None),
				Err(Error::InvalidTradingPathLength),
			);
            assert_eq!(
				uniswap.get_supply_amounts(&[DOT, PDEX].to_vec(), 24962, None),
				Ok(vec![10000, 24962])
			);
            assert_eq!(
				uniswap.get_supply_amounts(&[DOT, PDEX].to_vec(), 25000, None),
				Ok(vec![10031, 25000])
			);
            assert_eq!(
				uniswap.get_supply_amounts(&[DOT, PDEX, BTC].to_vec(), 10000, None),
				Err(Error::ZeroSupplyAmount),
			);
			assert_eq!(
				uniswap.get_supply_amounts(&[DOT, BTC].to_vec(), 10000, None),
				Err(Error::InsufficientLiquidity),
			);
        }

        #[ink::test]
        fn get_amount_for_big_number_work() {
            let mut uniswap = UniswapV2::new();

            assert_eq!(
                uniswap.add_liquidity(PDEX, DOT, 171_000_000_000_000_000_000_000, 56_000_000_000_000_000_000_000, 0, true),
                Ok(())
            );

            assert_eq!(
				uniswap.get_supply_amount(
                    171_000_000_000_000_000_000_000,
                    56_000_000_000_000_000_000_000,
                    1_000_000_000_000_000_000_000
                ),
				3_118_446_247_834_412_327_893,
			);

            assert_eq!(
				uniswap.get_target_amount(
                    171_000_000_000_000_000_000_000,
                    56_000_000_000_000_000_000_000,
                    3_118_446_247_834_412_327_893
                ),
				1_000_000_000_000_000_000_000,
			);
        }

        #[ink::test]
        fn _swap_work() {
            let mut uniswap = UniswapV2::new();

            assert_eq!(
                uniswap.add_liquidity(PDEX, DOT, 50000, 10000, 0, true),
                Ok(())
            );

            assert_eq!(uniswap._swap(PDEX, DOT, 50000, 5001), Err(Error::InvariantCheckFailed));
            assert_eq!(uniswap._swap(PDEX, DOT, 50000, 5000), Ok(()));
            assert_eq!(uniswap.get_liquidity(PDEX, DOT), (100000, 5000));
            assert_eq!(uniswap._swap(DOT, PDEX, 100, 800), Ok(()));
            assert_eq!(uniswap.get_liquidity(PDEX, DOT), (99200, 5100));
        }

        #[ink::test]
        fn _swap_by_path_work() {
            let mut uniswap = UniswapV2::new();

            assert_eq!(uniswap.add_liquidity(PDEX, DOT, 50000, 10000, 0, true), Ok(()));
            assert_eq!(uniswap.add_liquidity(PDEX, BTC, 100000, 10, 0, true), Ok(()));

            assert_eq!(uniswap.get_liquidity(PDEX, DOT), (50000, 10000));
            assert_eq!(uniswap.get_liquidity(PDEX, BTC), (100000, 10));

            assert_eq!(uniswap._swap_by_path(&[DOT, PDEX].to_vec(), &[10000, 25000].to_vec()), Ok(()));
            assert_eq!(uniswap.get_liquidity(PDEX, DOT), (25000, 20000));

            assert_eq!(uniswap._swap_by_path(&[DOT, PDEX, BTC].to_vec(), &[100000, 20000, 1].to_vec()), Ok(()));
            assert_eq!(uniswap.get_liquidity(PDEX, DOT), (5000, 120000));
            assert_eq!(uniswap.get_liquidity(PDEX, BTC), (120000, 9));
        }
    }
}