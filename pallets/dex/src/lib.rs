#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_support::sp_runtime::offchain::storage_lock::BlockNumberProvider;
use frame_support::traits::Get;
use frame_system::ensure_signed;
use pallet_generic_asset::AssetIdProvider;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_arithmetic::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, UniqueSaturatedFrom};
use sp_std::collections::btree_map;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::convert::TryInto;
use sp_std::str;
use sp_std::vec::Vec;

use crate::engine::{Order, OrderBook};

pub mod binary_heap;
mod engine;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
pub mod apis;

/// Configure the pallet by specifying the parameters and types on which it depends.
/// pallet_generic_asset::Trait bounds this DEX pallet with pallet_generic_asset. DEX is available
/// only for runtimes that also install pallet_generic_asset.
pub trait Trait: frame_system::Trait + pallet_generic_asset::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type UNIT: Get<<Self as pallet_generic_asset::Trait>::Balance>;
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId,
	 Balance = <T as pallet_generic_asset::Trait>::Balance,
	  BlockNumber = <T as frame_system::Trait>::BlockNumber{
	    /// Calculated Market Order Quantity
	    CalculatedOrderAmount(FixedU128),
	    /// Complete Fill Sell Order [orderId,filled_amount]
	    CompleteFillSell(sp_std::vec::Vec<u8>,FixedU128),
	    /// Partial Fill Sell Order [orderId,filled_amount]
	    PartialFillSell(sp_std::vec::Vec<u8>,FixedU128),
	    /// Complete Fill Buy Order [orderId,filled_amount]
	    CompleteFillBuy(sp_std::vec::Vec<u8>,FixedU128),
	    /// Partial Fill Buy Order [orderId,filled_amount]
	    PartialFillBuy(sp_std::vec::Vec<u8>,FixedU128),
        /// Market Buy Completion [filled_amount]
        MarketBuy(FixedU128),
        /// Market Sell Completion [filled_amount]
        MarketSell(FixedU128),
		/// Internal Error
		InternalError,
        /// New order added to AsksHeap [price,quantity]
        NewAskOrderAdded(FixedU128,FixedU128),
		/// New order added to BidsHeap [price,quantity]
		NewBidOrderAdded(FixedU128,FixedU128),
		/// There is a price level match in BidsHeap [bidsOrder,currentOrder]
		PriceLevelMatchBidsHeap(FixedU128,FixedU128),
		/// There is a price level match in AsksHeap [bidsOrder,currentOrder]
		PriceLevelMatchAsksHeap(FixedU128,FixedU128),
		/// Triggered when asks.peek() returns None
		AsksHeapEmpty,
		/// Triggered when bids.peek() returns None
		BidsHeapEmpty,
		/// The traded amount
		TradeAmount(Balance, FixedU128, AccountId),
		/// Not enough asset free balance for placing the trade
		InsufficientAssetBalance(FixedU128),
		/// Order contains a duplicate orderId of another active order
		DuplicateOrderId(Vec<u8>),
		/// Order type of Order is None
		OrderTypeIsNone,
		/// Price and Quantity cannot be zero
		PriceOrQuanitityIsZero,
		/// Invalid TradingPair Id
		TradingPairNotFound(u32),
		/// Same Assets cannot be traded
		SameAssetIdsError(u32,u32),
		/// Zero Balances in either one or both Assets
		NoBalanceOfAssets(u128,u128),
		/// When a new TradingPair is created
		TradingPairCreated(u32),
		/// New Order created
		NewOrderCreated(Vec<u8>,engine::OrderType,FixedU128,FixedU128,AccountId,BlockNumber),
		/// Contains market state about current block.
		/// Order: tradingPair,blockNumber,opening_bid,opening_ask,closing_bid,closing_ask,volume
		MarketData(u32,u32,FixedU128,FixedU128,FixedU128,FixedU128,FixedU128),
		// FIXME( Currently we iterate over all the trading pairs and emit events which is expensive)
		// TODO: Emit Market Data for only those markets which changed during the block.
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Error occured due to a Overflow during calculation
		CalculationOverflow,
		/// Order Failed to pass basic order checks
		BasicOrderChecksFailed,
		/// Same assets cannot be traded
		SameAssetIdsError,
		/// Zero Balance in both assets during registration
		InsufficientAssetBalance
	}
}

decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as TemplateModule {
		/// Storage items related to DEX Starts here
		Books get(fn books): map hasher(blake2_128_concat) u32 => engine::OrderBook<T::AccountId,T::BlockNumber,T::AssetId>;

		BookId get(fn book_id): u32;
	}
}


// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		// TODO: Note for enabling feeless trades use dispatch::DispatchResultWithPostInfo
		// TODO: then in the Ok(()) replace it with Ok(Some(0).into()) to make it fee-less

		/// This is used to list a new trading pair in the DEX. The origin has to reserve the
		/// TokenListingFee + PairListingFee if the token is not already available in DEX else
		/// only the PairListingFee is reserved until the token is de-listed from the DEX.
		/// Origin will not have any interest. It will avoid abusing the DEX with invaluable tokens
		/// and trading pairs.
		/// trading pair notation: trading_asset/base_asset
		#[weight = 10000]
		pub fn register_new_orderbook(origin, trading_asset_id: u32, base_asset_id: u32) -> dispatch::DispatchResultWithPostInfo{
		let _trader = ensure_signed(origin)?;
		// TODO: Add Error on registering same tradingPairs.


		/// If assets ids are same then it's error
		if &trading_asset_id == &base_asset_id {
		Self::deposit_event(RawEvent::SameAssetIdsError(trading_asset_id, base_asset_id));
		return Err(<Error<T>>::SameAssetIdsError.into());
		}
		/// The origin should have a non-zero balance in either one asset.
		let trading_asset_balance = pallet_generic_asset::Module::<T>::free_balance(&Self::u32_to_asset_id(trading_asset_id), &_trader);
		let base_asset_balance = pallet_generic_asset::Module::<T>::free_balance(&Self::u32_to_asset_id(base_asset_id), &_trader);
		if (TryInto::<u128>::try_into(trading_asset_balance).ok().unwrap()>0) || (TryInto::<u128>::try_into(base_asset_balance).ok().unwrap()>0){
		/// The origin should reserve a certain amount of SpendingAssetCurrency for registering the pair

		if Self::reserve_balance_registration(&_trader){
		/// Create the orderbook
		let trading_pair_id = Self::create_order_book(Self::u32_to_asset_id(trading_asset_id),Self::u32_to_asset_id(base_asset_id));
		Self::deposit_event(RawEvent::TradingPairCreated(trading_pair_id));
		return Ok(Some(0).into());
		}else{
		return Err(<Error<T>>::InsufficientAssetBalance.into());
		}
		}else{
		// If the balance of either one asset of trading pair is non zero, return error.
		Self::deposit_event(RawEvent::NoBalanceOfAssets(TryInto::<u128>::try_into(trading_asset_balance).ok().unwrap(),
		TryInto::<u128>::try_into(base_asset_balance).ok().unwrap()));
		return Err(<Error<T>>::InsufficientAssetBalance.into());
		}
		}

		/// This function can be used to submit limit orders
		/// Trading pair notation: trading_asset/base_asset ie (BTC/USDT)
		/// Price is BTC/USDT and Quantity is BTC
		#[weight = 10000]
		pub fn submit_order(origin,
		  order_type: engine::OrderType,
		  order_id: sp_std::vec::Vec<u8>,
		  price: FixedU128,
		  quantity: FixedU128,
		  trading_pair: u32) -> dispatch::DispatchResultWithPostInfo{
		let trader = ensure_signed(origin)?;

		match Self::basic_order_checks(&trader,trading_pair,price,quantity,order_type.clone(),order_id.clone()){

		Some(mut order_book) => {
		// TODO: Update the market data struct
		order_book = Self::execute_normal_order(order_book,order_type.clone(),order_id.clone(),price,quantity,&trader);
		<Books<T>>::insert(trading_pair,order_book); // Modifies the state to insert new order_book
		         },
		None => {
		return Err(<Error<T>>::BasicOrderChecksFailed.into());
		        }
		    }
		Ok(Some(0).into())
		}

		/// This function can be used to cancel orders
		#[weight = 10000]
		pub fn cancel_order(origin, order_id: sp_std::vec::Vec<u8>, trading_pair: u32) -> dispatch::DispatchResult{
		let _trader = ensure_signed(origin)?;
		// TODO: Do the cancel order logic for the given orderID.
		Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
    /// Initializes new Orderbook struct and saves it to storage with given current_id as key.
    fn create_order_book(trading_asset_id: T::AssetId, base_asset_id: T::AssetId) -> u32 {
        let current_id = Self::book_id();
        let current_block_num = <frame_system::Module<T>>::current_block_number();
        let order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> = engine::OrderBook {
            id: current_id,
            trading_asset: trading_asset_id,
            base_asset: base_asset_id,
            nonce: 0,
            orders: btree_map::BTreeMap::new(),
            advanced_bid_orders: binary_heap::BinaryHeap::new(),
            advanced_ask_orders: binary_heap::BinaryHeap::new_min(),
            bids: binary_heap::BinaryHeap::new(),
            asks: binary_heap::BinaryHeap::new_min(),
            // There should be only 28800 items or 1 day of blocks in this market_data vector
            market_data: sp_std::vec![engine::MarketData{
                current_block:  current_block_num,
                closing_bid: FixedU128::from(0),
                closing_ask: FixedU128::from(0),
                volume: FixedU128::from(0)
            }],
            enabled: true,
        };
        let tradingpair = order_book.id.clone();
        BookId::put(current_id + 1);
        Books::<T>::insert(order_book.id as u32, order_book);
        return tradingpair;
    }


    /// Reserves UNIT (defined in configuration trait) balance of SpendingAssetCurrency
    fn reserve_balance_registration(origin: &<T as frame_system::Trait>::AccountId) -> bool {
        pallet_generic_asset::Module::<T>::reserve(
            &pallet_generic_asset::SpendingAssetIdProvider::<T>::asset_id(),
            origin, <T as Trait>::UNIT::get()).is_ok()
    }

    // Converts Rust primitive type u32 to pallet_generic_asset type AssetId
    fn u32_to_asset_id(input: u32) -> T::AssetId {
        input.into()
    }

    /// Checks trading pair
    /// Checks balance
    /// Checks order id
    /// Checks order_type for Valid order type
    /// Checks if price & quantity is Zero
    /// Provides Orderbook for modification, reducing calls to storage
    /// Note: Price is in (base_asset/trading_asset) and Quantity is in trading_asset
    /// Trading pair notation: trading_asset/base_asset ie (BTC/USDT)
    /// Price is BTC/USDT and Quantity is BTC
    /// Note orderbook is only retrieved from storage if all the checks are passed.
    /// Reads and Writes to Substrate Storage is super expensive. :-|
    fn basic_order_checks(origin: &<T as frame_system::Trait>::AccountId, trading_pair: u32,
                          price: FixedU128, quantity: FixedU128, order_type: engine::OrderType,
                          order_id: sp_std::vec::Vec<u8>) -> Option<engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId>> {
        if order_type == engine::OrderType::None {
            Self::deposit_event(RawEvent::OrderTypeIsNone);
            return None;
        }
        match order_type {
            engine::OrderType::AskLimit | engine::OrderType::BidLimit => {
                if price <= FixedU128::from(0) || quantity <= FixedU128::from(0) {
                    Self::deposit_event(RawEvent::PriceOrQuanitityIsZero);
                    return None;
                }
            }
            engine::OrderType::BidMarket => {
                if price <= FixedU128::from(0) {
                    Self::deposit_event(RawEvent::PriceOrQuanitityIsZero);
                    return None;
                }
            }
            engine::OrderType::AskMarket => {
                if quantity <= FixedU128::from(0) {
                    Self::deposit_event(RawEvent::PriceOrQuanitityIsZero);
                    return None;
                }
            }
            _ => {
                return None;
            }
        }
        if !(<Books<T>>::contains_key(trading_pair)) {
            Self::deposit_event(RawEvent::TradingPairNotFound(trading_pair));
            return None;
        }

        let order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> = <Books<T>>::get(trading_pair);

        let trading_asset_id = order_book.get_trading_asset();
        let base_asset_id = order_book.get_base_asset();
        let orders = order_book.get_orders();

        match order_type {
            engine::OrderType::AskLimit | engine::OrderType::AskMarket => {
                // Check if that much quantity is available
                let trading_balance = pallet_generic_asset::Module::<T>::free_balance(&trading_asset_id, &origin);
                if let Some(trading_balance_converted) = Self::convert_balance_to_fixed_u128(trading_balance) {
                    if Self::has_balance_for_trading(orders, trading_balance_converted, quantity, order_id) {
                        Some(order_book)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            engine::OrderType::BidLimit => {
                //  Check if price*quantity is available in the base_asset.
                let base_balance = pallet_generic_asset::Module::<T>::free_balance(&base_asset_id, &origin);
                if let Some(base_balance_converted) = Self::convert_balance_to_fixed_u128(base_balance) {
                    if let Some(computed_trade_amount) = (&price).checked_mul(&quantity) {
                        if Self::has_balance_for_trading(orders, base_balance_converted, computed_trade_amount, order_id) {
                            Some(order_book)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            engine::OrderType::BidMarket => {
                // There is no balance check available for BidMarket
                // as Price is determined only during trade execution
                let base_balance = pallet_generic_asset::Module::<T>::free_balance(&base_asset_id, &origin);
                if let Some(base_balance_converted) = Self::convert_balance_to_fixed_u128(base_balance) {
                    if base_balance_converted >= price {
                        Some(order_book)
                    } else {
                        Self::deposit_event(RawEvent::InsufficientAssetBalance(base_balance_converted));
                        None
                    }
                } else {
                    None
                }
            }
            _ => {
                None
            }
        }
    }

    /// Checks if the given order id exists in the given orderbook
    fn check_order_id(orders: &btree_map::BTreeMap<Vec<u8>, engine::Order<T::AccountId, T::BlockNumber>>
                      , order_id: sp_std::vec::Vec<u8>) -> bool {
        if orders.contains_key(&order_id) { // TODO: This check is not working, do something better!!!
            Self::deposit_event(RawEvent::DuplicateOrderId(order_id));
            false
        } else {
            true
        }
    }

    /// Checks if the balance is enough to execute given trade and returns the orderbook
    fn has_balance_for_trading(orders: &btree_map::BTreeMap<Vec<u8>, engine::Order<T::AccountId, T::BlockNumber>>,
                               balance_to_check: FixedU128,
                               computed_amount: FixedU128,
                               order_id: sp_std::vec::Vec<u8>)
                               -> bool {
        return if balance_to_check >= computed_amount {
            if Self::check_order_id(&orders, order_id) {
                true
            } else {
                false
            }
        } else {
            Self::deposit_event(RawEvent::InsufficientAssetBalance(balance_to_check));
            false
        };
    }

    /// Converts Balance to FixedU128 representation
    pub fn convert_balance_to_fixed_u128(x: T::Balance) -> Option<FixedU128> {
        if let Some(y) = TryInto::<u128>::try_into(x).ok() {
            FixedU128::from(y).checked_div(&FixedU128::from(1_000_000_000_000))
        } else {
            None
        }
    }

    /// Converts FixedU128 to Balance representation
    pub fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<T::Balance> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(UniqueSaturatedFrom::<u128>::unique_saturated_from(balance_in_u128))
        } else {
            None
        }
    }

    /// The trading algorithm lives here
    /// The idea is that if the incoming order is Bidlimit then load the asks(minimum Binaryheap)
    /// and check if there are any others that are at the same price or lower than bidLimit order's price
    /// if so take those asks orders in a FIFO fashion and execute against Bidlimit order
    /// In case of no matching prices or asks is empty or BidLimit order is partially fulfilled
    /// then push Bidlimit order to it's pricelevel in bids (maximum BinaryHeap).
    ///
    /// For AskLimit orders... load the bids (maximum BinaryHeap) and try to find price that are same
    /// or greater than AskLimit order's price, then execute in FIFO fashion. Finally similar to Bidlimit
    /// in case of not matching prices or empty bids BinaryHeap or AskLimit order is partially fulfilled
    /// add it to asks at it's pricelevel.
    ///
    ///
    /// For Market Orders, it is similar to BidLimit and AskLimit respectively, only difference is
    /// in case of order partially filled or no matching price level or corresponding BinaryHeap's empty
    /// then it will not be added to orderbook but discarded with a event to notify the users.
    fn execute_normal_order(mut order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId>,
                            order_type: engine::OrderType,
                            order_id: sp_std::vec::Vec<u8>,
                            price: FixedU128,
                            quantity: FixedU128,
                            trader: &<T as frame_system::Trait>::AccountId)
                            -> OrderBook<T::AccountId, T::BlockNumber, T::AssetId> {
        // TODO: There might be lot of problems with borrow and reference management
        // TODO: A lot of checking is required
        let mut current_order: engine::Order<T::AccountId, T::BlockNumber> = engine::Order {
            id: order_id.clone(),
            order_type: order_type.clone(),
            price: price.clone(),
            quantity: quantity.clone(),
            market_maker: false,
            origin: trader.clone(),
            expiry: T::BlockNumber::default(), // We ignore this expiry as it is only used for advanced orders
        };
        let trading_asset_id = *order_book.get_trading_asset();
        let base_asset_id = *order_book.get_base_asset();
        match order_type {
            // Buy Limit Order
            engine::OrderType::BidLimit => {
                // Reserve the price*quantity of base-asset
                let trade_amount: T::Balance = Self::convert_fixed_u128_to_balance(
                    current_order.get_price().checked_mul(
                        current_order.get_quantity()).unwrap()).unwrap(); // TODO: Take care of these unwraps
                let _ = pallet_generic_asset::Module::<T>::reserve(&base_asset_id,
                                                                   current_order.get_origin(),
                                                                   trade_amount); // It will never fail

                let asks = order_book.get_asks_mut();
                loop {
                    if let Some(counter_price_level_peek) = asks.peek() {
                        if counter_price_level_peek.get_price_level() <= &price {
                            if let Some(mut counter_price_level) = asks.pop() {
                                // There are orders and counter_price_level matches asked_price_level
                                let orders = counter_price_level.get_orders_mut();
                                let mut matched = false;

                                while let Some(mut counter_order) = orders.pop_front() {
                                    let counter_quantity = counter_order.get_quantity();
                                    if counter_quantity > current_order.get_quantity() {
                                        // partially execute counter order
                                        counter_order = Self::partially_execute_order(counter_order,
                                                                                      &current_order.clone(),
                                                                                      trading_asset_id.clone(),
                                                                                      base_asset_id.clone(),
                                                                                      false,
                                                                                      &current_order.get_order_type());
                                        // push front the remaining counter order
                                        orders.push_front(counter_order);
                                        matched = true;
                                        break;
                                    } else if counter_quantity == current_order.get_quantity() {
                                        // fully execute current order
                                        // fully execute counter order
                                        Self::fully_execute_order(counter_order,
                                                                  &current_order.clone(),
                                                                  trading_asset_id.clone(),
                                                                  base_asset_id.clone(),
                                                                  false,
                                                                  &current_order.get_order_type());
                                        // Remove both orders
                                        matched = true;
                                        break;
                                    } else {
                                        // partially execute current order
                                        current_order = Self::partially_execute_order(current_order.clone(),
                                                                                      &counter_order,
                                                                                      trading_asset_id.clone(),
                                                                                      base_asset_id.clone(),
                                                                                      true,
                                                                                      &current_order.get_order_type());
                                        // pop another order from queue or insert new bid in bids
                                    }
                                }

                                if matched {
                                    // current order executed completely
                                    // save the state and exit
                                    // If there are orders left in this price level put it back in the BinaryHeap
                                    if counter_price_level.get_orders().len() != 0 {
                                        asks.push(counter_price_level);
                                    }
                                    break;
                                }
                            } else {
                                // It will not occur as peek will be None if price level is not there
                            }
                        } else {
                            // There are orders but not at asked price_level so add this order to bids
                            // current_order is market maker so no fees
                            let current_order_cloned = current_order.clone();
                            order_book = Self::add_to_bids(order_book, current_order);
                            Self::deposit_event(RawEvent::NewBidOrderAdded(*current_order_cloned.get_price(), *current_order_cloned.get_quantity()));
                            break;
                        }
                    } else {
                        // There are no orders in the heap so add this order to bids
                        // current_order is market maker so no fees
                        // There are orders but not at asked price_level so add this order to bids
                        // current_order is market maker so no fees
                        let current_order_cloned = current_order.clone();
                        order_book = Self::add_to_bids(order_book, current_order);
                        Self::deposit_event(RawEvent::NewBidOrderAdded(*current_order_cloned.get_price(), *current_order_cloned.get_quantity()));
                        break;
                    }
                }

                order_book
            }
            // Sell Limit Order
            engine::OrderType::AskLimit => {
                // Reserve the price*quantity of base-asset
                let quantity: T::Balance = Self::convert_fixed_u128_to_balance(
                    *current_order.get_quantity()).unwrap(); // TODO: Take care of these unwraps
                let _ = pallet_generic_asset::Module::<T>::reserve(&trading_asset_id,
                                                                   current_order.get_origin(),
                                                                   quantity); // It will never fail

                let bids = order_book.get_bids_mut();
                loop {
                    if let Some(counter_price_level_peek) = bids.peek() {
                        if counter_price_level_peek.get_price_level() >= &price {
                            if let Some(mut counter_price_level) = bids.pop() {
                                // There are orders and counter_price_level matches asked_price_level
                                let orders = counter_price_level.get_orders_mut();
                                let mut matched = false;
                                while let Some(mut counter_order) = orders.pop_front() {
                                    let counter_quantity = counter_order.get_quantity();
                                    if counter_quantity > current_order.get_quantity() {
                                        // partially execute counter order
                                        counter_order = Self::partially_execute_order(counter_order,
                                                                                      &current_order.clone(),
                                                                                      trading_asset_id.clone(),
                                                                                      base_asset_id.clone(),
                                                                                      true,
                                                                                      &current_order.get_order_type());
                                        // push front the remaining counter order
                                        orders.push_front(counter_order);
                                        matched = true;
                                        break;
                                    } else if counter_quantity == current_order.get_quantity() {
                                        // fully execute current order
                                        // fully execute counter order
                                        Self::fully_execute_order(counter_order,
                                                                  &current_order.clone(),
                                                                  trading_asset_id.clone(),
                                                                  base_asset_id.clone(),
                                                                  true,
                                                                  &current_order.get_order_type());
                                        // Remove both orders
                                        matched = true;
                                        break;
                                    } else {
                                        // partially execute current order
                                        // TODO: Price taken for calculation maybe wrong. Check it
                                        current_order = Self::partially_execute_order(current_order.clone(),
                                                                                      &counter_order,
                                                                                      trading_asset_id.clone(),
                                                                                      base_asset_id.clone(),
                                                                                      false,
                                                                                      &current_order.get_order_type());
                                        // pop another order from queue or insert new bid in bids
                                    }
                                }
                                if matched {
                                    // current order executed completely
                                    // save the state and exit
                                    // If there are orders left in this price level put it back in the BinaryHeap
                                    if !counter_price_level.get_orders().is_empty() {
                                        bids.push(counter_price_level);
                                    }
                                    break;
                                }
                            } else {
                                // It will not occur as peek will be None if price level is not there
                            }
                        } else {
                            // There are orders but not at asked price_level so add this order to bids
                            // current_order is market maker so no fees
                            let current_order_cloned = current_order.clone();
                            order_book = Self::add_to_asks(order_book, current_order);
                            Self::deposit_event(RawEvent::NewAskOrderAdded(*current_order_cloned.get_price(), *current_order_cloned.get_quantity()));
                            break;
                        }
                    } else {
                        // There are no orders in the heap so add this order to bids
                        // current_order is market maker so no fees
                        // There are orders but not at asked price_level so add this order to bids
                        // current_order is market maker so no fees
                        let current_order_cloned = current_order.clone();
                        order_book = Self::add_to_asks(order_book, current_order);
                        Self::deposit_event(RawEvent::NewAskOrderAdded(*current_order_cloned.get_price(), *current_order_cloned.get_quantity()));
                        break;
                    }
                }

                order_book
            }
            // Buy Market Order
            engine::OrderType::BidMarket => {
                // In this case current_order.price stores the total amount in base-asset for which market order is executed.
                // TODO: Take care of reserving and un-reserving of assets
                let amount_filled = FixedU128::from(0);
                let asks = order_book.get_asks_mut();
                loop {
                    if let Some(mut counter_price_level) = asks.pop() {
                        // There are orders and counter_price_level matches asked_price_level
                        let counter_price = *counter_price_level.get_price_level();
                        let orders = counter_price_level.get_orders_mut();
                        let mut matched = false;
                        match current_order.get_price()
                            .checked_div(&counter_price) {
                            Some(amount) => {
                                current_order.quantity = amount;
                                Self::deposit_event(RawEvent::CalculatedOrderAmount(amount));
                            }
                            None => {
                                Self::deposit_event(RawEvent::InternalError);
                                return order_book;
                            }
                        }

                        while let Some(mut counter_order) = orders.pop_front() {
                            let counter_quantity = counter_order.get_quantity();
                            if counter_quantity > current_order.get_quantity() {
                                // partially execute counter order
                                amount_filled.checked_add(current_order.get_quantity());
                                counter_order = Self::partially_execute_order(counter_order,
                                                                              &current_order.clone(),
                                                                              trading_asset_id.clone(),
                                                                              base_asset_id.clone(),
                                                                              true,
                                                                              &current_order.get_order_type());
                                // push front the remaining counter order
                                orders.push_front(counter_order);
                                matched = true;
                                break;
                            } else if counter_quantity == current_order.get_quantity() {
                                // fully execute current order
                                // fully execute counter order
                                amount_filled.checked_add(current_order.get_quantity());
                                Self::fully_execute_order(counter_order,
                                                          &current_order,
                                                          trading_asset_id.clone(),
                                                          base_asset_id.clone(),
                                                          true,
                                                          &current_order.get_order_type());
                                // Remove both orders
                                matched = true;
                                break;
                            } else {
                                // partially execute current order
                                amount_filled.checked_add(counter_order.get_quantity());
                                current_order = Self::partially_execute_order(current_order.clone(),
                                                                              &counter_order,
                                                                              trading_asset_id.clone(),
                                                                              base_asset_id.clone(),
                                                                              false,
                                                                              &current_order.get_order_type());
                                // pop another order from queue or insert new bid in bids
                            }
                        }

                        if matched {
                            // current order executed completely
                            // save the state and exit
                            // If there are orders left in this price level put it back in the BinaryHeap
                            if !counter_price_level.get_orders().is_empty() {
                                asks.push(counter_price_level);
                            }
                            break;
                        }
                    } else {
                        // No price levels are available AskHeap is empty
                        Self::deposit_event(RawEvent::AsksHeapEmpty);
                        Self::deposit_event(RawEvent::MarketBuy(amount_filled));
                        return order_book;
                    }
                }

                if amount_filled > FixedU128::from(0) {
                    Self::deposit_event(RawEvent::MarketBuy(amount_filled));
                }
                order_book
            }
            // Sell Market Order
            engine::OrderType::AskMarket => {
                // In this case current_order.quantity contains the market quantity that should be sold
                let amount_filled = FixedU128::from(0);
                let bids = order_book.get_bids_mut();
                loop {
                    if let Some(mut counter_price_level) = bids.pop() {
                        // There are orders and counter_price_level matches asked_price_level
                        let orders = counter_price_level.get_orders_mut();
                        let mut matched = false;
                        while let Some(mut counter_order) = orders.pop_front() {
                            let counter_quantity = counter_order.get_quantity();
                            if counter_quantity > current_order.get_quantity() {
                                // partially execute counter order
                                amount_filled.checked_add(current_order.get_quantity());
                                counter_order = Self::partially_execute_order(counter_order,
                                                                              &current_order.clone(),
                                                                              trading_asset_id.clone(),
                                                                              base_asset_id.clone(),
                                                                              true,
                                                                              &current_order.get_order_type());
                                // push front the remaining counter order
                                orders.push_front(counter_order);
                                matched = true;
                                break;
                            } else if counter_quantity == current_order.get_quantity() {
                                // fully execute current order
                                // fully execute counter order
                                amount_filled.checked_add(current_order.get_quantity());
                                Self::fully_execute_order(counter_order,
                                                          &current_order,
                                                          trading_asset_id.clone(),
                                                          base_asset_id.clone(),
                                                          true,
                                                          &current_order.get_order_type());
                                // Remove both orders
                                matched = true;
                                break;
                            } else {
                                // partially execute current order
                                amount_filled.checked_add(counter_order.get_quantity());
                                current_order = Self::partially_execute_order(current_order.clone(),
                                                                              &counter_order,
                                                                              trading_asset_id.clone(),
                                                                              base_asset_id.clone(),
                                                                              false,
                                                                              &current_order.get_order_type());
                                // pop another order from queue or insert new bid in bids
                            }
                        }

                        if matched {
                            // current order executed completely
                            // save the state and exit
                            // If there are orders left in this price level put it back in the BinaryHeap
                            if !counter_price_level.get_orders().is_empty() {
                                bids.push(counter_price_level);
                            }
                            break;
                        }
                    } else {
                        // There are not orders in the BidHeap
                        Self::deposit_event(RawEvent::BidsHeapEmpty);
                        Self::deposit_event(RawEvent::MarketSell(amount_filled));
                        break;
                    }
                }

                if amount_filled > FixedU128::from(0) {
                    Self::deposit_event(RawEvent::MarketSell(amount_filled));
                }
                order_book
            }
            // TODO:  Ignores other cases maybe should we generate an event for it?
            _ => {
                order_book
            }
        }
    }

    /// For the sake of understanding
    fn partially_execute_order(executing_order: engine::Order<T::AccountId, T::BlockNumber>,
                               trigger_order: &engine::Order<T::AccountId, T::BlockNumber>,
                               trading_asset_id: T::AssetId,
                               base_asset_id: T::AssetId,
                               take_price_from_executing_order: bool,
                               order_type: &engine::OrderType) -> Order<T::AccountId, T::BlockNumber> {

        return Self::fully_execute_order(executing_order, trigger_order, trading_asset_id, base_asset_id, take_price_from_executing_order, order_type);
    }

    /// Here we un-reserve the corresponding assets of both buyer and seller and transfers their
    /// balances according trade amount or quantity.
    fn fully_execute_order(mut executing_order: engine::Order<T::AccountId, T::BlockNumber>,
                           trigger_order: &engine::Order<T::AccountId, T::BlockNumber>,
                           trading_asset_id: T::AssetId,
                           base_asset_id: T::AssetId,
                           take_price_from_executing_order: bool, // It is used to differentiate incoming order
                                                                  // and counter order from orderbook
                           order_type: &engine::OrderType) -> Order<T::AccountId, T::BlockNumber> {
        return match order_type { // TODO: Check this
            engine::OrderType::BidLimit => {
                // TODO: Remove the unwraps it can cause a panic
                let trade_amount: T::Balance;
                if take_price_from_executing_order {
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        executing_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                } else {
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        trigger_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                }
                let trigger_quantity: T::Balance = Self::convert_fixed_u128_to_balance(*trigger_order.get_quantity()).unwrap();
                // un-reserve price*quantity of base_asset from executing_order's origin
                pallet_generic_asset::Module::<T>::unreserve(&base_asset_id,
                                                             executing_order.get_origin(),
                                                             trade_amount);
                // un-reserve quantity of trading_asset from trigger_order's origin
                pallet_generic_asset::Module::<T>::unreserve(&trading_asset_id,
                                                             trigger_order.get_origin(),
                                                             trigger_quantity);
                // TODO: Check the results for OK()
                // Transfer price*quantity of base_asset from executing_order's origin to trigger_order's origin
                let _result = pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                               executing_order.get_origin(),
                                                                               trigger_order.get_origin(),
                                                                               trade_amount);
                // Transfer quantity of trading_asset from trigger_order's origin to executing_order's origin
                let _result = pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                               trigger_order.get_origin(),
                                                                               executing_order.get_origin(),
                                                                               trigger_quantity);

                // TODO: Remove the unwraps it can cause a panic
                executing_order.set_quantity(executing_order.get_quantity().checked_sub(trigger_order.get_quantity()).unwrap());
                // TODO: Deposit events for partial fill of executing_order
                // TODO: Deposit events for complete fill of trigger_order
                executing_order
            }
            engine::OrderType::AskLimit => {
                let trade_amount: T::Balance;
                // TODO: Take care of unwraps
                if take_price_from_executing_order {
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        executing_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                } else {
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        trigger_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                }
                let trigger_quantity: T::Balance = Self::convert_fixed_u128_to_balance(*trigger_order.get_quantity()).unwrap(); // TODO: Remove the unwraps it can cause a panic
                // un-reserve quantity of trading_asset from executing_order's origin
                pallet_generic_asset::Module::<T>::unreserve(&trading_asset_id,
                                                             executing_order.get_origin(),
                                                             trigger_quantity);
                // un-reserve price*quantity of base_asset from trigger_order's origin
                pallet_generic_asset::Module::<T>::unreserve(&base_asset_id,
                                                             trigger_order.get_origin(),
                                                             trade_amount);
                // TODO: Check the results for OK()
                // Transfer quantity of trading_asset from executing_order's origin to trigger_order's origin
                let _result = pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                               executing_order.get_origin(),
                                                                               trigger_order.get_origin(),
                                                                               trigger_quantity);
                // Transfer price*quantity of base_asset from trigger_order's origin to executing_order's origin
                let _result = pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                               trigger_order.get_origin(),
                                                                               executing_order.get_origin(),
                                                                               trade_amount);

                // TODO: Remove the unwraps it can cause a panic
                executing_order.set_quantity(executing_order.get_quantity().checked_sub(trigger_order.get_quantity()).unwrap());
                // TODO: Deposit events for partial fill of executing_order
                // TODO: Deposit events for complete fill of trigger_order
                executing_order
            }
            engine::OrderType::BidMarket => {
                let trade_amount: T::Balance;
                let trigger_quantity: T::Balance = Self::convert_fixed_u128_to_balance(*trigger_order.get_quantity()).unwrap();
                if take_price_from_executing_order {
                    // Here executing is counterOrder (Sell Limit) and trigger is currentOrder( Market Buy)
                    // When counterOrder.quantity >= currentOrder.quantity
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        executing_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                    pallet_generic_asset::Module::<T>::unreserve(&trading_asset_id,
                                                                 executing_order.get_origin(),
                                                                 trigger_quantity);
                    pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                     trigger_order.get_origin(),
                                                                     executing_order.get_origin(),
                                                                     trade_amount);
                    pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                     executing_order.get_origin(),
                                                                     trigger_order.get_origin(),
                                                                     trigger_quantity);
                } else {
                    // Here trigger is counterOrder (Sell Limit) and executing is currentOrder( Market Buy)
                    // When counterOrder.quantity < currentOrder.quantity
                    trade_amount = Self::convert_fixed_u128_to_balance(
                        trigger_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                    pallet_generic_asset::Module::<T>::unreserve(&trading_asset_id,
                                                                 trigger_order.get_origin(),
                                                                 trigger_quantity);
                    pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                     executing_order.get_origin(),
                                                                     trigger_order.get_origin(),
                                                                     trade_amount);
                    pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                     trigger_order.get_origin(),
                                                                     executing_order.get_origin(),
                                                                     trigger_quantity);
                }

                // TODO: Remove the unwraps it can cause a panic
                executing_order.set_quantity(executing_order.get_quantity().checked_sub(trigger_order.get_quantity()).unwrap());
                if take_price_from_executing_order {
                    let order_id = executing_order.get_id();
                    if executing_order.get_quantity() == &FixedU128::from(0) {
                        // Deposit events for complete fill of Sell Limit
                        Self::deposit_event(RawEvent::CompleteFillSell(order_id.clone(), *executing_order.get_quantity()));
                    } else {
                        // Deposit events for partial fill of Sell Limit
                        Self::deposit_event(RawEvent::PartialFillSell(order_id.clone(), *executing_order.get_quantity()));
                    }
                } else {
                    let order_id = trigger_order.get_id();
                    if trigger_order.get_quantity() == &FixedU128::from(0) {
                        // Deposit events for complete fill of Sell Limit
                        Self::deposit_event(RawEvent::CompleteFillSell(order_id.clone(), *trigger_order.get_quantity()));
                    } else {
                        // Deposit events for partial fill of Sell Limit
                        Self::deposit_event(RawEvent::PartialFillSell(order_id.clone(), *trigger_order.get_quantity()));
                    }
                }
                executing_order
            }
            engine::OrderType::AskMarket => {
                let trade_quantity: T::Balance;
                if take_price_from_executing_order {
                    // here executing_order is counter_order (Bid Limit) and trigger_order is current_order (Market Sell)
                    // When counterOrder.quantity >= currentOrder.quantity
                    trade_quantity = Self::convert_fixed_u128_to_balance(*trigger_order.get_quantity()).unwrap();
                    let trade_amount = Self::convert_fixed_u128_to_balance(
                        executing_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                    pallet_generic_asset::Module::<T>::unreserve(&base_asset_id,
                                                                 executing_order.get_origin(),
                                                                 trade_amount);

                    pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                     executing_order.get_origin(),
                                                                     trigger_order.get_origin(),
                                                                     trade_amount);
                    pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                     trigger_order.get_origin(),
                                                                     executing_order.get_origin(),
                                                                     trade_quantity);
                } else {
                    // here executing_order is current_order (Market Sell) and trigger_order is counter_order (Bid Limit)
                    // When counterOrder.quantity < currentOrder.quantity
                    trade_quantity = Self::convert_fixed_u128_to_balance(*executing_order.get_quantity()).unwrap();
                    let trade_amount = Self::convert_fixed_u128_to_balance(
                        trigger_order.get_price().checked_mul(trigger_order.get_quantity()).unwrap()).unwrap();
                    pallet_generic_asset::Module::<T>::unreserve(&base_asset_id,
                                                                 trigger_order.get_origin(),
                                                                 trade_amount);

                    pallet_generic_asset::Module::<T>::make_transfer(&base_asset_id,
                                                                     trigger_order.get_origin(),
                                                                     executing_order.get_origin(),
                                                                     trade_amount);
                    pallet_generic_asset::Module::<T>::make_transfer(&trading_asset_id,
                                                                     executing_order.get_origin(),
                                                                     trigger_order.get_origin(),
                                                                     trade_quantity);
                }
                executing_order.set_quantity(executing_order.get_quantity().checked_sub(trigger_order.get_quantity()).unwrap());
                if take_price_from_executing_order {
                    let order_id = executing_order.get_id();
                    if executing_order.get_quantity() == &FixedU128::from(0) {
                        // Deposit events for complete fill of Buy Limit
                        Self::deposit_event(RawEvent::CompleteFillBuy(order_id.clone(), *executing_order.get_quantity()));
                    } else {
                        // Deposit events for partial fill of Buy Limit
                        Self::deposit_event(RawEvent::PartialFillBuy(order_id.clone(), *executing_order.get_quantity()));
                    }
                } else {
                    let order_id = trigger_order.get_id();
                    if trigger_order.get_quantity() == &FixedU128::from(0) {
                        // Deposit events for complete fill of Buy Limit
                        Self::deposit_event(RawEvent::CompleteFillBuy(order_id.clone(), *trigger_order.get_quantity()));
                    } else {
                        // Deposit events for partial fill of Buy Limit
                        Self::deposit_event(RawEvent::PartialFillBuy(order_id.clone(), *trigger_order.get_quantity()));
                    }
                }
                executing_order
            }
            _ => {
                // Ignore other patterns
                executing_order
            }
        };
    }

    /// Adds the given current_order to the bids of given orderbook.
    /// P.S. : Super Slow implementation :-\
    fn add_to_bids(mut order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId>,
                   current_order: Order<T::AccountId, T::BlockNumber>)
                   -> engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> {
        Self::deposit_event(RawEvent::AsksHeapEmpty); // TODO: Better Naming of Events. It can also be Order not matched in AsksHeap
        let bids = order_book.clone().get_bids();
        let mut price_level_match = true;
        let mut bids_sorted_vec = bids.into_sorted_vec(); //TODO: CHANGE THIS: This is going to be super duper expensive.
        for index in 0..bids_sorted_vec.len() {
            if bids_sorted_vec[index].get_price_level() != current_order.get_price() {
                // current_order price and price_level doesn't match
                // so we need to create one and put it in
                price_level_match = false;
            } else {
                // Price and price_level matches
                // Add it to queue of this price_level
                Self::deposit_event(RawEvent::PriceLevelMatchBidsHeap(*bids_sorted_vec[index].get_price_level(),
                                                                      *current_order.get_price()));
                bids_sorted_vec[index].get_orders_mut().push_back(current_order.clone());
                price_level_match = true;
            }
        }

        //TODO: CHANGE THIS: This is going to be super duper expensive.
        let mut modified_bids = binary_heap::BinaryHeap::from(bids_sorted_vec);

        if modified_bids.is_empty() && price_level_match == true {
            // There are no price levels available
            // Create one and put it in
            Self::deposit_event(RawEvent::BidsHeapEmpty);
            let mut new_price_level = engine::PriceLevel {
                price_level: *current_order.get_price(),
                queue: VecDeque::new(),
            };
            new_price_level.get_orders_mut().push_back(current_order.clone());
            modified_bids.push(new_price_level);
        }
        if !price_level_match && !modified_bids.is_empty() {
            let mut new_price_level = engine::PriceLevel {
                price_level: *current_order.get_price(),
                queue: VecDeque::new(),
            };
            new_price_level.get_orders_mut().push_back(current_order.clone());
            modified_bids.push(new_price_level);
        }

        // let mut bids_mut = order_book.get_bids_mut();
        // bids_mut = modified_bids;
        order_book.bids = modified_bids;
        order_book
    }

    /// Adds the given current_order to the asks of given orderbook.
    /// P.S. : Super Slow implementation :-\
    fn add_to_asks(mut order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId>,
                   current_order: Order<T::AccountId, T::BlockNumber>)
                   -> engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> {
        Self::deposit_event(RawEvent::BidsHeapEmpty); // TODO: Better Naming of Events. It can be Order not matched in BidsHeap
        let asks = order_book.clone().get_asks();
        let mut price_level_match = true;
        let mut asks_sorted_vec = asks.into_sorted_vec(); //TODO: This is going to be super duper expensive.
        for index in 0..asks_sorted_vec.len() {
            if asks_sorted_vec[index].get_price_level() != current_order.get_price() {
                // current_order price and price_level doesn't match
                // so we need to create one and put it in
                price_level_match = false;
            } else {
                // Price and price_level matches
                // Add it to queue of this price_level
                Self::deposit_event(RawEvent::PriceLevelMatchAsksHeap(*asks_sorted_vec[index].get_price_level(),
                                                                      *current_order.get_price()));
                asks_sorted_vec[index].get_orders_mut().push_back(current_order.clone());
                price_level_match = true;
            }
        }

        //TODO: This is going to be super duper expensive.
        let mut modified_asks = binary_heap::BinaryHeap::from_vec_cmp(asks_sorted_vec, binary_heap::MinComparator);

        if modified_asks.is_empty() && price_level_match == true {
            // There are no price levels available
            // Create one and put it in
            Self::deposit_event(RawEvent::AsksHeapEmpty);
            let mut new_price_level = engine::PriceLevel {
                price_level: *current_order.get_price(),
                queue: VecDeque::new(),
            };
            new_price_level.get_orders_mut().push_back(current_order.clone());
            modified_asks.push(new_price_level);
        }
        if !price_level_match && !modified_asks.is_empty() {
            let mut new_price_level = engine::PriceLevel {
                price_level: *current_order.get_price(),
                queue: VecDeque::new(),
            };
            new_price_level.get_orders_mut().push_back(current_order.clone());
            modified_asks.push(new_price_level);
        }

        // let mut bids_mut = order_book.get_bids_mut();
        // bids_mut = modified_bids;
        order_book.asks = modified_asks;
        order_book
    }

    /// Helper function
    pub fn calculate_trade_amount(price: &FixedU128, quantity: &FixedU128) -> T::Balance {
        let trade_amount: T::Balance = Self::convert_fixed_u128_to_balance(
            price.checked_mul(
                quantity).unwrap()).unwrap(); // TODO: Take care of these unwraps
        return trade_amount;
    }

    /// Helper function
    pub fn get_user_balance(who: &<T as frame_system::Trait>::AccountId, asset_id: T::AssetId) -> Option<FixedU128> {
        Self::convert_balance_to_fixed_u128(pallet_generic_asset::Module::<T>::free_balance(&asset_id, who))
    }

    /// Helper function
    pub fn get_order_book_testing(trading_pair: u32) -> engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> {
        let order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> = <Books<T>>::get(trading_pair);
        return order_book;
    }

    /// Helper function for implementing RPC that gets the orderbook from storage
    pub fn get_order_book(trading_pair: u32) -> apis::OrderBookApi {
        if !(<Books<T>>::contains_key(trading_pair)) {
            let order_book_data = apis::OrderBookApi {
                bids: Vec::with_capacity(0),
                asks: Vec::with_capacity(0),
                enabled: false,
            };
            return order_book_data;
        }
        let order_book: engine::OrderBook<T::AccountId, T::BlockNumber, T::AssetId> = <Books<T>>::get(trading_pair);

        let mut order_book_data = apis::OrderBookApi {
            bids: Vec::with_capacity(10),
            asks: Vec::with_capacity(10),
            enabled: true,
        };
        // Computing Asks Amount
        let mut asks = order_book.asks;
        for _ in 0..10 {
            if let Some(price_level) = asks.pop() {
                let mut quantity = FixedU128::from(0);
                let orders: Vec<&Order<T::AccountId, T::BlockNumber>> = price_level.get_orders().iter().collect();
                for order in orders {
                    quantity = quantity.saturating_add(order.quantity)
                }
                let new_price_level = apis::PriceLevelData {
                    price_level: *price_level.get_price_level(),
                    amount: quantity,
                };
                order_book_data.asks.push(new_price_level);
            } else {
                break;
            }
        }
        // Computing Bids Amount
        let mut bids = order_book.bids;
        for _ in 0..10 {
            if let Some(price_level) = bids.pop() {
                let mut quantity = FixedU128::from(0);
                let orders: Vec<&Order<T::AccountId, T::BlockNumber>> = price_level.get_orders().iter().collect();
                for order in orders {
                    quantity = quantity.saturating_add(order.quantity)
                }
                let new_price_level = apis::PriceLevelData {
                    price_level: *price_level.get_price_level(),
                    amount: quantity,
                };
                order_book_data.bids.push(new_price_level);
            } else {
                break;
            }
        }
        return order_book_data;
    }
}

/// Macro related to accessing orderbook from Substrate Runtime for RPC
sp_api::decl_runtime_apis! {
    pub trait DexRuntimeApi {
        fn get_order_book(trading_pair: u32) -> apis::OrderBookApi;
    }

}