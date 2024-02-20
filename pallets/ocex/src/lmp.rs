use crate::{
	pallet::{IngressMessages, PriceOracle, TraderMetrics, TradingPairs},
	storage::OffchainState,
	BalanceOf, Config, Error, LMPEpoch, Pallet,
};
use frame_support::dispatch::DispatchResult;
use orderbook_primitives::{
	types::{OrderSide, Trade, TradingPair},
	LiquidityMining,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ocex::TradingPairConfig, AccountId, UNIT_BALANCE};
use rust_decimal::{
	prelude::{ToPrimitive, Zero},
	Decimal,
};
use sp_runtime::{traits::BlockNumberProvider, DispatchError, SaturatedConversion};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub fn update_trade_volume_by_main_account(
	state: &mut OffchainState,
	epoch: u16,
	market: &TradingPairConfig,
	volume: Decimal,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let trading_pair = TradingPair::from(market.quote_asset, market.base_asset);
	let key = (epoch, trading_pair, "trading_volume", main).encode();
	Ok(match state.get(&key)? {
		None => {
			state.insert(key, volume.encode());
			volume
		},
		Some(encoded_volume) => {
			let recorded_volume = Decimal::decode(&mut &encoded_volume[..])
				.map_err(|_| "Unable to decode decimal")?;
			let total = recorded_volume.saturating_add(volume);
			state.insert(key, total.encode());
			total
		},
	})
}

#[allow(dead_code)]
pub fn get_trade_volume_by_main_account(
	state: &mut OffchainState,
	epoch: u16,
	trading_pair: &TradingPair,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let key = (epoch, trading_pair, "trading_volume", main).encode();
	Ok(match state.get(&key)? {
		None => Decimal::zero(),
		Some(encoded_volume) => {
			let recorded_volume = Decimal::decode(&mut &encoded_volume[..])
				.map_err(|_| "Unable to decode decimal")?;
			recorded_volume
		},
	})
}

pub fn get_maker_volume_by_main_account(
	state: &mut OffchainState,
	epoch: u16,
	trading_pair: &TradingPair,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let key = (epoch, trading_pair, "maker_volume", main).encode();
	Ok(match state.get(&key)? {
		None => Decimal::zero(),
		Some(encoded_volume) => {
			Decimal::decode(&mut &encoded_volume[..]).map_err(|_| "Unable to decode decimal")?
		},
	})
}

pub fn update_maker_volume_by_main_account(
	state: &mut OffchainState,
	epoch: u16,
	market: &TradingPairConfig,
	volume: Decimal,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let trading_pair = TradingPair::from(market.quote_asset, market.base_asset);
	let key = (epoch, trading_pair, "maker_volume", main).encode();
	Ok(match state.get(&key)? {
		None => {
			state.insert(key, volume.encode());
			volume
		},
		Some(encoded_volume) => {
			let recorded_volume = Decimal::decode(&mut &encoded_volume[..])
				.map_err(|_| "Unable to decode decimal")?;
			let total = recorded_volume.saturating_add(volume);
			state.insert(key, total.encode());
			total
		},
	})
}

pub fn store_fees_paid_by_main_account_in_quote(
	state: &mut OffchainState,
	epoch: u16,
	market: &TradingPairConfig,
	fees_in_quote_terms: Decimal,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let trading_pair = TradingPair::from(market.quote_asset, market.base_asset);
	let key = (epoch, trading_pair, "fees_paid", main).encode();
	Ok(match state.get(&key)? {
		None => {
			state.insert(key, fees_in_quote_terms.encode());
			fees_in_quote_terms
		},
		Some(encoded_fees_paid) => {
			let recorded_fees_paid = Decimal::decode(&mut &encoded_fees_paid[..])
				.map_err(|_| "Unable to decode decimal")?;
			let total_fees = recorded_fees_paid.saturating_add(fees_in_quote_terms);
			state.insert(key, total_fees.encode());
			total_fees
		},
	})
}

pub fn get_fees_paid_by_main_account_in_quote(
	state: &mut OffchainState,
	epoch: u16,
	trading_pair: &TradingPair,
	main: &AccountId,
) -> Result<Decimal, &'static str> {
	let key = (epoch, trading_pair, "fees_paid", main).encode();
	Ok(match state.get(&key)? {
		None => Decimal::zero(),
		Some(encoded_fees_paid) => {
			Decimal::decode(&mut &encoded_fees_paid[..]).map_err(|_| "Unable to decode decimal")?
		},
	})
}

pub fn store_q_score_and_uptime(
	state: &mut OffchainState,
	epoch: u16,
	index: u16,
	score: Decimal,
	trading_pair: &TradingPair,
	main: &AccountId,
) -> Result<(), &'static str> {
	let key = (epoch, trading_pair, "q_score&uptime", main).encode();
	match state.get(&key)? {
		None => state.insert(key, BTreeMap::from([(index, score)]).encode()),
		Some(encoded_q_scores_map) => {
			let mut map = BTreeMap::<u16, Decimal>::decode(&mut &encoded_q_scores_map[..])
				.map_err(|_| "Unable to decode decimal")?;
			if map.insert(index, score).is_some() {
				log::error!(target:"ocex","Overwriting q score with index: {:?}, epoch: {:?}, main: {:?}, market: {:?}",index,epoch,main,trading_pair);
				return Err("Overwriting q score");
			}
			state.insert(key, map.encode());
		},
	}
	Ok(())
}

/// Returns the total Q score and uptime
pub fn get_q_score_and_uptime(
	state: &mut OffchainState,
	epoch: u16,
	trading_pair: &TradingPair,
	main: &AccountId,
) -> Result<(Decimal, u16), &'static str> {
	let key = (epoch, trading_pair, "q_score&uptime", main).encode();
	match state.get(&key)? {
		None => {
			log::error!(target:"ocex","q_score&uptime not found for: main: {:?}, market: {:?}",main, trading_pair);
			Err("Q score not found")
		},
		Some(encoded_q_scores_map) => {
			let map = BTreeMap::<u16, Decimal>::decode(&mut &encoded_q_scores_map[..])
				.map_err(|_| "Unable to decode decimal")?;
			let mut total_score = Decimal::zero();
			// Add up all individual scores
			for score in map.values() {
				total_score = total_score.saturating_add(*score);
			}
			Ok((total_score, map.len() as u16))
		},
	}
}

impl<T: Config> Pallet<T> {
	pub fn update_lmp_storage_from_trade(
		state: &mut OffchainState,
		trade: &Trade,
		config: TradingPairConfig,
		maker_fees: Decimal,
		taker_fees: Decimal,
	) -> Result<(), &'static str> {
		let epoch: u16 = <LMPEpoch<T>>::get();

		// Store trade.price * trade.volume as maker volume for this epoch
		let volume = trade.price.saturating_mul(trade.amount);
		update_trade_volume_by_main_account(
			state,
			epoch,
			&config,
			volume,
			&trade.maker.main_account,
		)?;
		update_trade_volume_by_main_account(
			state,
			epoch,
			&config,
			volume,
			&trade.taker.main_account,
		)?;
		update_maker_volume_by_main_account(
			state,
			epoch,
			&config,
			volume,
			&trade.maker.main_account,
		)?;

		// Store maker_fees and taker_fees for the corresponding main account for this epoch
		match trade.maker.side {
			OrderSide::Ask => {
				let fees = maker_fees; // Maker fees is in quote because they put ask order.
				store_fees_paid_by_main_account_in_quote(
					state,
					epoch,
					&config,
					fees,
					&trade.maker.main_account,
				)?;

				// Convert taker fees to quote terms based on trade price
				let fees = taker_fees.saturating_mul(trade.price);
				store_fees_paid_by_main_account_in_quote(
					state,
					epoch,
					&config,
					fees,
					&trade.taker.main_account,
				)?;
			},
			OrderSide::Bid => {
				// Convert maker fees to quote terms based on trade price
				let fees = maker_fees.saturating_mul(trade.price);
				store_fees_paid_by_main_account_in_quote(
					state,
					epoch,
					&config,
					fees,
					&trade.maker.main_account,
				)?;

				// Taker fees is in quote because they put bid order.
				let fees = taker_fees.saturating_mul(trade.price);
				store_fees_paid_by_main_account_in_quote(
					state,
					epoch,
					&config,
					fees,
					&trade.taker.main_account,
				)?;
			},
		}
		Ok(())
	}

	/// Returns the top scored lmp account for the given epoch and market.
	pub fn top_lmp_accounts(
		epoch: u16,
		trading_pair: TradingPair,
		sorted_by_mm_score: bool,
		limit: usize,
	) -> Vec<T::AccountId> {
		let mut accounts: BTreeMap<Decimal, T::AccountId> = BTreeMap::new();
		let prefix = (epoch, trading_pair);
		for (main, (mm_score, trading_score, _)) in <TraderMetrics<T>>::iter_prefix(prefix) {
			if sorted_by_mm_score {
				accounts.insert(mm_score, main);
			} else {
				accounts.insert(trading_score, main);
			}
		}

		let mut accounts = accounts.values().cloned().collect::<Vec<T::AccountId>>();
		accounts.reverse(); // We want descending order

		if accounts.len() > limit {
			// Limit the items returned to top 'limit' accounts
			accounts = accounts.split_at(limit).0.to_vec()
		}

		accounts
	}
}

impl<T: Config> LiquidityMining<T::AccountId, BalanceOf<T>> for Pallet<T> {
	fn register_pool(pool_id: T::AccountId, trading_account: T::AccountId) -> DispatchResult {
		Self::register_user(pool_id, trading_account)
	}

	fn average_price(market: TradingPair) -> Option<Decimal> {
		let prices = <PriceOracle<T>>::get();
		prices.get(&(market.base, market.quote)).map(|(price, _ticks)| *price)
	}

	fn is_registered_market(market: &TradingPair) -> bool {
		<TradingPairs<T>>::contains_key(market.base, market.quote)
	}

	fn add_liquidity(
		market: TradingPair,
		pool: T::AccountId,
		lp: T::AccountId,
		total_shares_issued: Decimal,
		base_amount: Decimal,
		quote_amount: Decimal,
	) -> DispatchResult {
		let unit = Decimal::from(UNIT_BALANCE);
		let base_amount_in_u128 = base_amount
			.saturating_mul(unit)
			.to_u128()
			.ok_or(Error::<T>::FailedToConvertDecimaltoBalance)?;
		Self::do_deposit(pool.clone(), market.base, base_amount_in_u128.saturated_into())?;
		let quote_amount_in_u128 = quote_amount
			.saturating_mul(unit)
			.to_u128()
			.ok_or(Error::<T>::FailedToConvertDecimaltoBalance)?;
		Self::do_deposit(pool.clone(), market.quote, quote_amount_in_u128.saturated_into())?;
		let current_blk = frame_system::Pallet::<T>::current_block_number();
		<IngressMessages<T>>::mutate(current_blk, |messages| {
			messages.push(polkadex_primitives::ingress::IngressMessages::AddLiquidity(
				TradingPairConfig::default(market.base, market.quote),
				pool,
				lp,
				total_shares_issued,
				base_amount,
				quote_amount,
			));
		});
		Ok(())
	}

	fn remove_liquidity(
		market: TradingPair,
		pool: T::AccountId,
		lp: T::AccountId,
		burned: BalanceOf<T>,
		total: BalanceOf<T>,
	) {
		let burned = Decimal::from(burned.saturated_into::<u128>());
		let total = Decimal::from(total.saturated_into::<u128>());
		let burn_frac = burned.checked_div(total).unwrap_or_default();

		let current_blk = frame_system::Pallet::<T>::current_block_number();
		<IngressMessages<T>>::mutate(current_blk, |messages| {
			messages.push(polkadex_primitives::ingress::IngressMessages::RemoveLiquidity(
				TradingPairConfig::default(market.base, market.quote),
				pool,
				lp,
				burn_frac,
				total,
			));
		});
	}

	fn force_close_pool(market: TradingPair, pool: T::AccountId) {
		let current_blk = frame_system::Pallet::<T>::current_block_number();
		<IngressMessages<T>>::mutate(current_blk, |messages| {
			messages.push(polkadex_primitives::ingress::IngressMessages::ForceClosePool(
				TradingPairConfig::default(market.base, market.quote),
				pool,
			));
		});
	}

	fn claim_rewards(
		main: T::AccountId,
		epoch: u16,
		market: TradingPair,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::do_claim_lmp_rewards(main, epoch, market)
	}
}
