#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_support::traits::Get;
use frame_system::ensure_signed;
use pallet_generic_asset::AssetIdProvider;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_arithmetic::traits::{CheckedDiv, CheckedMul, CheckedSub, UniqueSaturatedFrom, CheckedAdd};
use sp_runtime::traits::Hash;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::convert::TryInto;
use sp_std::str;
use sp_std::vec::Vec;
use frame_support::sp_tracing::tracing::stdlib::thread::current;

#[test]
mod mock;

#[test]
mod tests;


/// Configure the pallet by specifying the parameters and types on which it depends.
/// pallet_generic_asset::Trait bounds this DEX pallet with pallet_generic_asset. DEX is available
/// only for runtimes that also install pallet_generic_asset.
pub trait Trait: frame_system::Trait + pallet_generic_asset::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// Amount in SpendingAssetCurrency that must reserved to register a tradingPair
    type TradingPairReservationFee: Get<<Self as pallet_generic_asset::Trait>::Balance>;
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where Hash = <T as frame_system::Trait>::Hash{
		/// New Trading pair is created [TradingPairHash]
		TradingPairCreated(Hash),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Transaction contained Same AssetID for both base and quote.
		SameAssetIdsError,
		/// TradingPair already exists in the system
		TradingPairIDExists,
		/// Insufficent Balance to Execute
		InsufficientAssetBalance,
		/// Invalid Price or Quantity for a Limit Order
		InvalidPriceOrQuantityLimit,
		/// Invalid Price for a BidMarket Order
		InvalidBidMarketPrice,
		/// Invalid Quantity for a AskMarket Order
		InvalidAskMarketQuantity,
		/// TradingPair doesn't Exist
		InvalidTradingPair,
		/// Internal Error: Failed to Convert Balance to U128
		InternalErrorU128Balance,
	}
}


decl_storage! {

	trait Store for Module<T: Trait> as DEXModule {
	// Stores all the different price levels for all the trading pairs in a DoubleMap.
	PriceLevels get(fn get_pricelevels): double_map hasher(identity) T::Hash, hasher(blake2_128_concat) FixedU128 => LinkedPriceLevel<T>;
	// Stores all the different active ask and bid levels in the system as a sorted vector mapped to it's TradingPair.
	// Regarding Performance using sort_unstable(), it is in O(nlogn).
	AsksLevels get(fn get_askslevels): map hasher(identity) T::Hash => Vec<FixedU128>;
	BidsLevels get(fn get_bidslevels): map hasher(identity) T::Hash => Vec<FixedU128>;
	// Stores the Orderbook struct for all available trading pairs.
	Orderbooks get(fn get_orderbooks): map hasher(identity) T::Hash => Orderbook<T>;
	// Store MarketData of TradingPairs
	// If the market data is returning None, then no trades were present for that trading in that block.
	// TODO: Currently we store market data for all the blocks
	MarketInfo get(fn get_marketdata): double_map hasher(identity) T::Hash, hasher(blake2_128_concat) T::BlockNumber => Option<MarketData>;
	Nonce: u128;
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

		/// Registers a new trading pair in the system
		#[weight = 10000]
		pub fn register_new_orderbook(origin, quote_asset_id: u32, base_asset_id: u32) -> dispatch::DispatchResultWithPostInfo{
		    let trader = ensure_signed(origin)?;

		    // If assets ids are same then it's error
		    if &quote_asset_id == &base_asset_id {
		        return Err(<Error<T>>::SameAssetIdsError.into());
		    }

		    // Checks the tradingPair whether exists
		    let trading_pair_id = Self::create_trading_pair_id(&quote_asset_id,&base_asset_id);
		    if <Orderbooks<T>>::contains_key(&trading_pair_id){
		        return Err(<Error<T>>::TradingPairIDExists.into());
		    }

		    // The origin should reserve a certain amount of SpendingAssetCurrency for registering the pair
		    if Self::reserve_balance_registration(&trader){
		        // Create the orderbook
		        Self::create_order_book(quote_asset_id.into(),base_asset_id.into(),&trading_pair_id);
		        Self::deposit_event(RawEvent::TradingPairCreated(trading_pair_id));
		        return Ok(Some(0).into());
		    }else{
		        return Err(<Error<T>>::InsufficientAssetBalance.into());
		    }
	    }

        /// Submits the given order for matching to engine.
        #[weight = 10000]
	    pub fn submit_order(origin, order_type: OrderType, trading_pair: T::Hash, price: FixedU128, quantity: FixedU128) -> dispatch::DispatchResultWithPostInfo{
	        let trader = ensure_signed(origin)?;

	        Self::execute_order(trader, order_type, trading_pair, price, quantity); // TODO: It may an error in which case take the fees else refund
	        return Ok(Some(0).into());
	    }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum OrderType {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Order<T> where T: Trait {
    id: T::Hash,
    trading_pair: T::Hash,
    trader: T::AccountId,
    price: FixedU128,
    quantity: FixedU128,
    order_type: OrderType,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct LinkedPriceLevel<T> where T: Trait {
    next: Option<FixedU128>,
    prev: Option<FixedU128>,
    orders: VecDeque<Order<T>>,
}

impl<T> Default for LinkedPriceLevel<T> where T: Trait {
    fn default() -> Self {
        LinkedPriceLevel {
            next: None,
            prev: None,
            orders: Default::default(),
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct Orderbook<T> where T: Trait {
    trading_pair: T::Hash,
    base_asset_id: T::AssetId,
    quote_asset_id: T::AssetId,
    best_bid_price: FixedU128,
    best_ask_price: FixedU128,
}

impl<T> Default for Orderbook<T> where T: Trait {
    fn default() -> Self {
        Orderbook {
            trading_pair: T::Hash::default(),
            base_asset_id: 0.into(),
            quote_asset_id: 0.into(),
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0),
        }
    }
}

impl<T> Orderbook<T> where T: Trait {
    fn new(base_asset_id: T::AssetId, quote_asset_id: T::AssetId, trading_pair: T::Hash) -> Self {
        Orderbook {
            trading_pair,
            base_asset_id,
            quote_asset_id,
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0),
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct MarketData {
    // Lowest price at which the trade was executed in a block.
    low: FixedU128,
    // Highest price at which the trade was executed in a block.
    high: FixedU128,
    // Total volume traded in a block.
    volume: FixedU128,
}

impl<T: Trait> Module<T> {
    // Reserves TradingPairReservationFee (defined in configuration trait) balance of SpendingAssetCurrency
    fn reserve_balance_registration(origin: &<T as frame_system::Trait>::AccountId) -> bool {
        pallet_generic_asset::Module::<T>::reserve(
            &pallet_generic_asset::SpendingAssetIdProvider::<T>::asset_id(),
            origin, <T as Trait>::TradingPairReservationFee::get()).is_ok()
    }

    // Initializes a new Orderbook and stores it in the Orderbooks
    fn create_order_book(quote_asset_id: T::AssetId, base_asset_id: T::AssetId, trading_pair_id: &T::Hash) {
        let orderbook = Orderbook::new(base_asset_id, quote_asset_id, trading_pair_id.clone());
        <Orderbooks<T>>::insert(trading_pair_id, orderbook);
        <AsksLevels<T>>::insert(trading_pair_id, Vec::<FixedU128>::new());
        <BidsLevels<T>>::insert(trading_pair_id, Vec::<FixedU128>::new());
    }

    // Creates a TradingPairID from both Asset IDs.
    fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> T::Hash {
        (quote_asset_id, base_asset_id).using_encoded(<T as frame_system::Trait>::Hashing::hash)
    }

    // Submits an order for execution
    fn execute_order(trader: T::AccountId,
                     order_type: OrderType,
                     trading_pair: T::Hash,
                     price: FixedU128,
                     quantity: FixedU128) -> Option<Error<T>> {
        let mut current_order = Order {
            id: T::Hash::default(), // let's do the hashing after the checks.
            trading_pair,
            trader,
            price,
            quantity,
            order_type,
        };

        match Self::basic_order_checks(&current_order) {
            Ok(mut orderbook) => {
                let nonce = Nonce::get(); // To get some kind non user controllable randomness to order id
                current_order.id = (trading_pair, current_order.trader.clone(), price, quantity, current_order.order_type.clone(), nonce)
                    .using_encoded(<T as frame_system::Trait>::Hashing::hash);
                Nonce::put(nonce + 1); // TODO: It might overflow after a long time.

                match current_order.order_type {
                    OrderType::AskMarket | OrderType::BidMarket => {
                        (current_order, orderbook) = Self::consume_order(current_order, orderbook);
                    }

                    OrderType::AskLimit | OrderType::BidLimit => {
                        // Check if current can consume orders present in the system
                        if (current_order.order_type == OrderType::BidLimit &&
                            current_order.price >= orderbook.best_ask_price) |
                            (current_order.order_type == OrderType::AskLimit &&
                                current_order.price <= orderbook.best_bid_price) {

                            // current_order can consume i.e. Market Taking order
                            (current_order, orderbook) = Self::consume_order(current_order, orderbook);

                            // If current_order has quantity remaining to fulfil, insert it
                            if current_order.quantity > FixedU128::from(0) {
                                // Insert the remaining order in the order book
                                (current_order, orderbook) = Self::insert_order(current_order, orderbook);
                            }
                        } else {
                            // Current Order cannot be consumed i.e. Market Making order
                            // Insert the order in the order book
                            (current_order, orderbook) = Self::insert_order(current_order, orderbook)
                        }
                    }
                }
                // TODO: Save the orderbook back in into the storage
                // TODO: Finally emit the events about order execution
                None
            }
            Err(err_value) => {
                return Some(err_value);
            }
        }
    }

    // Inserts the given order into orderbook
    fn insert_order(current_order: Order<T>, mut orderbook: Orderbook<T>) -> (Order<T>, Orderbook<T>) {
        // TODO: bids_levels should be sorted in descending order
        // TODO: asks_levels should be sorted in ascending order
        // TODO: The logic given below is assuming that 0th index of bids_levels is highest bid price &
        // TODO: 0th index of asks_levels is lowest ask price.
        match current_order.order_type {
            OrderType::BidLimit => {
                // bids_levels contains the sorted pricelevels
                let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                match bids_levels.binary_search(&current_order.price) {
                    Ok(_) => {
                        // current_order.price is already there in the system
                        // so we just need to insert into it's linkedpricelevel FIFO.
                        let mut linked_pricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &current_order.price);
                        linked_pricelevel.orders.push_back(current_order.clone());
                        // Write it back to storage
                        <PriceLevels<T>>::insert(&current_order.trading_pair, &current_order.price, linked_pricelevel)
                        // Access there is not new price level creation, there is won't be any change to orderbook's best prices
                    }
                    Err(index_at_which_we_should_insert) => {
                        bids_levels.insert(index_at_which_we_should_insert, current_order.price);
                        // Here there can be three cases,
                        // 1. when current_order is the last item in the bids_levels
                        // 2. when current_order is the first item in the bids_levels
                        // 3. when current_order is inserted in between two other items in bids_levels
                        if index_at_which_we_should_insert != 0 && index_at_which_we_should_insert != bids_levels.len() - 1 {
                            // Third case
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert - 1).unwrap());
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert + 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*bids_levels.get(index_at_which_we_should_insert + 1).unwrap()),
                                prev: Some(*bids_levels.get(index_at_which_we_should_insert - 1).unwrap()),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert - 1).unwrap(),
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert + 1).unwrap(),
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }

                        if index_at_which_we_should_insert == 0 {
                            // Second Case
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert + 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*bids_levels.get(index_at_which_we_should_insert + 1).unwrap()),
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert + 1).unwrap(),
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                            // As current_order has the best_bid price, we store that to best_bid_price
                            orderbook.best_bid_price = current_order.price;
                        }
                        if index_at_which_we_should_insert == bids_levels.len() - 1 {
                            // First Case
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert - 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: Some(*bids_levels.get(index_at_which_we_should_insert - 1).unwrap()),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert - 1).unwrap(),
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }
                    }
                }
                <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);
            }
            OrderType::AskLimit => {
                // asks_levels contains the sorted pricelevels
                let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                match asks_levels.binary_search(&current_order.price) {
                    Ok(_) => {
                        // current_order.price is already there in the system
                        // so we just need to insert into it's linkedpricelevel FIFO.
                        let mut linked_pricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &current_order.price);
                        linked_pricelevel.orders.push_back(current_order.clone());
                        // Write it back to storage
                        <PriceLevels<T>>::insert(&current_order.trading_pair, &current_order.price, linked_pricelevel)
                        // Access there is not new price level creation, there is won't be any change to orderbook's best prices
                    }
                    Err(index_at_which_we_should_insert) => {
                        asks_levels.insert(index_at_which_we_should_insert, current_order.price);
                        // Here there can be three cases,
                        // 1. when current_order is the last item in the asks_levels
                        // 2. when current_order is the first item in the asks_levels
                        // 3. when current_order is inserted in between two other items in asks_levels
                        if index_at_which_we_should_insert != 0 && index_at_which_we_should_insert != asks_levels.len() - 1 {
                            // Third case
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert - 1).unwrap());
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert + 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*asks_levels.get(index_at_which_we_should_insert + 1).unwrap()),
                                prev: Some(*asks_levels.get(index_at_which_we_should_insert - 1).unwrap()),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert - 1).unwrap(),
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert + 1).unwrap(),
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }

                        if index_at_which_we_should_insert == 0 {
                            // Second Case
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert + 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*bids_levels.get(index_at_which_we_should_insert + 1).unwrap()),
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert + 1).unwrap(),
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);

                            // As current_order has the best_bid price, we store that to best_bid_price
                            orderbook.best_ask_price = current_order.price;
                        }
                        if index_at_which_we_should_insert == asks_levels.len() - 1 {
                            // First Case
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert - 1).unwrap());
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: Some(*asks_levels.get(index_at_which_we_should_insert - 1).unwrap()),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);

                            // All the value updates are done. Write it back to storage.
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert - 1).unwrap(),
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }
                    }
                }
                <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);
            }
            _ => {
                // It will never execute
            }
        }
        (current_order, orderbook)
    }

    fn consume_order(mut current_order: Order<T>, mut orderbook: Orderbook<T>) -> (Order<T>, Orderbook<T>) {
        let market_data: MarketData;
        if <MarketInfo<T>>::contains_key(&current_order.trading_pair, <frame_system::Module<T>>::block_number()) {
            market_data = <MarketInfo<T>>::get(&current_order.trading_pair, <frame_system::Module<T>>::block_number())
        } else {
            market_data = MarketData{
                low: FixedU128::from(0),
                high: FixedU128::from(0),
                volume: FixedU128::from(0)
            }
        }
        match current_order.order_type {
            OrderType::BidLimit => {
                // The incoming order is BidLimit and it will be able to match best_ask_price
                // Hence, we load the corresponding LinkedPriceLevel of best_ask_price from storage and execute

                // we want to match the orders until the current_price is less than the ask_price
                // or the current_order is fulfilled completely
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_ask_price);
                while current_order.quantity > FixedU128::from(0) {
                    if Some(counter_order) = linkedpricelevel.orders.pop_front() {
                        (current_order, counter_order, market_data) = Self::do_asset_exchange(current_order, // TODO: Rust Ownership issues
                                                                                 counter_order,
                                                                                 market_data,
                                                                                 orderbook.base_asset_id,
                                                                                 orderbook.quote_asset_id);
                        if counter_order.quantity > FixedU128::from(0) {
                            // counter_order was not completely used so we store it back in the FIFO
                            linkedpricelevel.orders.push_front(counter_order);
                        }
                    } else {
                        // TODO: Check: Not sure if "no orders remaining" is the only case that will trigger this branch
                        // As no more orders are available in the linkedpricelevel.
                        // we check if we can match with the next available level

                        // As we consumed the linkedpricelevel completely remove that from asks_levels
                        let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                        // asks_levels is already sorted and the best_ask_price should be the first item
                        // so we don't need to sort it after we remove and simply remove the 0th index.
                        // NOTE: In asks_levels & bids_levels all items are unique.
                        asks_levels.remove(0);
                        // Write it back to storage.
                        <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);

                        if linkedpricelevel.next.is_none() {
                            // No more price levels available
                            break;
                        }

                        if current_order.price >= linkedpricelevel.next.unwrap() {
                            // In this case current_order.quantity is remaining and
                            // it can match with next price level in orderbook.

                            // Last best_ask_price is consumed and doesn't exist anymore hence
                            // we set new best_ask_price in orderbook.
                            orderbook.best_ask_price = linkedpricelevel.next.unwrap();
                            linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.unwrap());
                        } else {
                            // In this case, the current_order cannot match with the best_ask_price available
                            // so let's break the while loop and return the current_order and orderbook
                            break;
                        }
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    // Save Pricelevel back to storage
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_ask_price, linkedpricelevel);
                }
                return (current_order, orderbook);
            }

            OrderType::BidMarket => {
                // Incoming order is a Market buy order so it, trader whats to buy the quote_asset for
                // current_order.price at Market price.


                // We load the best_ask_price level and start to fill the order
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_ask_price);
                // We iterate until current_order is fulfilled or exhausts the Ask orders in the system.
                while current_order.price > FixedU128::from(0) {
                    if Some(counter_order) = linkedpricelevel.orders.pop_front() {
                        (current_order, counter_order,market_data) = Self::do_asset_exchange_market(current_order,
                                                                                        counter_order,
                                                                                        market_data,
                                                                                        orderbook.base_asset_id,
                                                                                        orderbook.quote_asset_id);

                        if counter_order.quantity > FixedU128::from(0) {
                            // counter_order was not completely used so we store it back in the FIFO
                            linkedpricelevel.orders.push_front(counter_order);
                        }
                    } else {
                        // TODO: Check: Not sure if "no orders remaining" is the only case that will trigger this branch
                        // As no more orders are available in the linkedpricelevel.
                        // we check if we can match with the next available level

                        // As we consumed the linkedpricelevel completely remove that from asks_levels
                        let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                        // asks_levels is already sorted and the best_ask_price should be the first item
                        // so we don't need to sort it after we remove and simply remove it
                        // NOTE: In asks_levels & bids_levels all items are unique.
                        asks_levels.remove(0);
                        // Write it back to storage.
                        <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);

                        if linkedpricelevel.next.is_none() {
                            // No more price levels available
                            break;
                        }

                        orderbook.best_ask_price = linkedpricelevel.next.unwrap();
                        linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.unwrap());
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    // Save Pricelevel back to storage
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_ask_price, linkedpricelevel);
                }

                return (current_order, orderbook);
            }

            OrderType::AskLimit => {
                // The incoming order is AskLimit and it will be able to match best_bid_price
                // Hence, we load the corresponding LinkedPriceLevel of best_bid_price from storage and execute

                // we want to match the orders until the current_price is greater than the bid_price
                // or the current_order is fulfilled completely
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_bid_price);
                while current_order.quantity > FixedU128::from(0) {
                    if Some(counter_order) = linkedpricelevel.orders.pop_front() {
                        (current_order, counter_order,market_data) = Self::do_asset_exchange(current_order, // TODO: Rust Ownership issues
                                                                                 counter_order,
                                                                                 market_data,
                                                                                 orderbook.base_asset_id,
                                                                                 orderbook.quote_asset_id);
                        if counter_order.quantity > FixedU128::from(0) {
                            // counter_order was not completely used so we store it back in the FIFO
                            linkedpricelevel.orders.push_front(counter_order);
                        }
                    } else {
                        // TODO: Check: Not sure if "no orders remaining" is the only case that will trigger this branch
                        // As no more orders are available in the linkedpricelevel.
                        // we check if we can match with the next available level

                        // As we consumed the linkedpricelevel completely remove that from bids_levels
                        let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                        // bids_levels is already sorted and the best_bid_price should be the first item
                        // so we don't need to sort it after we remove and simply remove it
                        // NOTE: In asks_levels & bids_levels all items are unique.
                        bids_levels.remove(0);
                        // Write it back to storage.
                        <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);

                        if linkedpricelevel.prev.is_none() {
                            // No more price levels available
                            break;
                        }
                        if current_order.price <= linkedpricelevel.prev.unwrap() {
                            // In this case current_order.quantity is remaining and
                            // it can match with next price level in orderbook.

                            // Last best_bid_price is consumed and doesn't exist anymore hence
                            // we set new best_bid_price in orderbook.
                            orderbook.best_bid_price = linkedpricelevel.prev.unwrap();
                            linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.prev.unwrap());
                        } else {
                            // In this case, the current_order cannot match with the best_bid_price available
                            // so let's break the while loop and return the current_order and orderbook
                            break;
                        }
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    // Save Pricelevel back to storage
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_bid_price, linkedpricelevel);
                }
                return (current_order, orderbook);
            }

            OrderType::AskMarket => {
                // Incoming Order is a Market Sell, so trader wants to sell current_order.quantity
                // at best possible price.
                // We load the best_bid_price level and start to fill the order
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_bid_price);
                while current_order.quantity > FixedU128::from(0) {
                    if Some(counter_order) = linkedpricelevel.orders.pop_front() {
                        (current_order, counter_order,market_data) = Self::do_asset_exchange_market(current_order,
                                                                                        counter_order,
                                                                                        market_data,
                                                                                        orderbook.base_asset_id,
                                                                                        orderbook.quote_asset_id);
                        if counter_order.quantity > FixedU128::from(0) {
                            // counter_order was not completely used so we store it back in the FIFO
                            linkedpricelevel.orders.push_front(counter_order);
                        }
                    } else {
                        // As no more orders are available in the linkedpricelevel.
                        // we check if we can match with the next available level

                        // As we consumed the linkedpricelevel completely remove that from bids_levels
                        let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                        // bids_levels is already sorted and the best_bid_price should be the first item
                        // so we don't need to sort it after we remove and simply remove it
                        // NOTE: In asks_levels & bids_levels all items are unique.
                        bids_levels.remove(0);
                        // Write it back to storage.
                        <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);

                        if linkedpricelevel.prev.is_none() {
                            // No more price levels available
                            break;
                        }

                        orderbook.best_bid_price = linkedpricelevel.prev.unwrap();
                        linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.prev.unwrap());
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    // Save Pricelevel back to storage
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_bid_price, linkedpricelevel);
                }

                return (current_order, orderbook);
            }
        }
        // Write the market data back to storage
        <MarketInfo<T>>::insert(&current_order.trading_pair, <frame_system::Module<T>>::block_number(),market_data);
        (current_order, orderbook)
    }

    fn do_asset_exchange_market(mut current_order: Order<T>, mut counter_order: Order<T>, mut market_data: MarketData, base_assetid: T::AssetId, quote_assetid: T::AssetId) -> (Order<T>, Order<T>,MarketData) {
        if market_data.low == FixedU128::from(0){
            market_data.low = counter_order.price
        }
        if market_data.high == FixedU128::from(0){
            market_data.high = counter_order.price
        }
        if market_data.high < counter_order.price {
            market_data.high = counter_order.price
        }
        if market_data.low > counter_order.price {
            market_data.low = counter_order.price
        }
        match current_order.order_type {
            OrderType::BidMarket => {
                let current_order_quantity = current_order.price.checked_div(&counter_order.price).unwrap();
                // 100/5 = 20.
                if current_order_quantity <= counter_order.quantity {
                    // We have enough quantity in the counter_order to fulfill current_order completely
                    // Transfer the base asset
                    Self::transfer_asset_market(base_assetid, current_order.price, &current_order.trader, &counter_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset(quote_assetid, current_order_quantity, &counter_order.trader, &current_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&current_order.price).unwrap();
                    //Set current_order quantity to 0 and counter_order is reduced by fulfilled amount
                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order_quantity).unwrap();
                    current_order.price = FixedU128::from(0);
                } else {
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).unwrap();
                    // Transfer the base asset
                    Self::transfer_asset_market(base_assetid, trade_amount, &current_order.trader, &counter_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset(quote_assetid, counter_order.quantity, &counter_order.trader, &current_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    // counter_order is set to 0 and current_order.price is reduced by fulfilled amount
                    counter_order.quantity = FixedU128::from(0);
                    current_order.price = current_order.price.checked_sub(&trade_amount).unwrap();
                }
            }
            OrderType::AskMarket => {
                if current_order.quantity <= counter_order.quantity {
                    // We have enough quantity in the counter_order to fulfill current_order completely
                    let trade_amount = counter_order.price.checked_mul(&current_order.quantity).unwrap();
                    // Transfer the base asset
                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset_market(quote_assetid, current_order.quantity, &current_order.trader, &counter_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    // current_order is set to 0 and counter_order is reduced by fulfilled amount
                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).unwrap();
                    current_order.quantity = FixedU128::from(0);
                } else {
                    // We have enough quantity in the counter_order to fulfill current_order completely
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).unwrap();
                    // Transfer the base asset
                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset_market(quote_assetid, counter_order.quantity, &current_order.trader, &counter_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    // counter_order is set to 0 and current_order is reduced by fulfilled amount
                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).unwrap();
                    counter_order.quantity = FixedU128::from(0);
                }
            }
            _ => {
                // It won't execute.
            }
        }
        (current_order, counter_order, market_data)
    }

    // It checks the if the counter_order.quantity has enough to fulfill current_order then exchanges
    // it after collecting fees and else if counter_order.quantity is less than current_order.quantity
    // then exchange counter_order.quantity completely and return.
    // current_order.quantity is modified to new value.
    fn do_asset_exchange(mut current_order: Order<T>, mut counter_order: Order<T>, mut market_data: MarketData, base_assetid: T::AssetId, quote_assetid: T::AssetId) -> (Order<T>, Order<T>,MarketData) {
        if market_data.low == FixedU128::from(0){
            market_data.low = counter_order.price
        }
        if market_data.high == FixedU128::from(0){
            market_data.high = counter_order.price
        }
        if market_data.high < counter_order.price {
            market_data.high = counter_order.price
        }
        if market_data.low > counter_order.price {
            market_data.low = counter_order.price
        }
        match current_order.order_type {
            OrderType::BidLimit => {
                // BTC/USDT - quote/base
                // The current order is trying to buy the quote_asset
                if current_order.quantity <= counter_order.quantity {
                    // We have enough quantity in the counter_order to fulfill current_order completely
                    // Calculate the total cost in base asset for buying required amount
                    let trade_amount = current_order.price.checked_mul(&current_order.quantity).unwrap();
                    // Transfer the base asset
                    // AssetId, amount to send, from, to
                    Self::transfer_asset(base_assetid, trade_amount, &current_order.trader, &counter_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset(quote_assetid, current_order.quantity, &counter_order.trader, &current_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    //Set Current order quantity to 0 and counter_order is subtracted.
                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).unwrap();
                    current_order.quantity = FixedU128::from(0);
                } else {
                    // current_order is partially filled and counter_order is completely filled.
                    // Calculate the total cost in base asset for buying required amount
                    let trade_amount = current_order.price.checked_mul(&counter_order.quantity).unwrap();
                    // Transfer the base asset
                    // AssetId, amount to send, from, to
                    Self::transfer_asset(base_assetid, trade_amount, &current_order.trader, &counter_order.trader);
                    // Transfer the quote asset from counter_order to current_order's trader.
                    Self::transfer_asset(quote_assetid, counter_order.quantity, &counter_order.trader, &current_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    //Set counter_order quantity to 0 and current_order is subtracted.
                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).unwrap();
                    counter_order.quantity = FixedU128::from(0);
                }
            }
            OrderType::AskLimit => {
                // The current order is trying to sell the quote_asset
                if current_order.quantity <= counter_order.quantity{
                    // We have enough quantity in the counter_order to fulfill current_order completely
                    // Calculate the total cost in base asset for selling the required amount
                    let trade_amount = counter_order.price.checked_mul(&current_order.quantity).unwrap();
                    // Transfer the base asset
                    // AssetId, amount to send, from, to
                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader);
                    // Transfer the quote asset
                    Self::transfer_asset(quote_assetid, current_order.quantity, &current_order.trader, &counter_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    //Set Current order quantity to 0 and counter_order is subtracted.
                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).unwrap();
                    current_order.quantity = FixedU128::from(0);
                }else{
                    // current_order is partially filled and counter_order is completely filled.
                    // Calculate the total cost in base asset for selling the required amount
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).unwrap();
                    // Transfer the base asset
                    // AssetId, amount to send, from, to
                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader);
                    // Transfer the quote asset from counter_order to current_order's trader.
                    Self::transfer_asset(quote_assetid, counter_order.quantity, &current_order.trader, &counter_order.trader);
                    // Add the volume executed
                    market_data.volume = market_data.volume.checked_add(&trade_amount).unwrap();
                    //Set counter_order quantity to 0 and current_order is subtracted.
                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).unwrap();
                    counter_order.quantity = FixedU128::from(0);
                }
            }
            _ => {
                // It will not execute
            }
        }
        (current_order, counter_order,market_data)
    }

    // Transfers the balance of traders
    fn transfer_asset(asset_id: T::AssetId, amount: FixedU128, from: &T::AccountId, to: &T::AssetId) {
        let amount_balance = Self::convert_fixed_u128_to_balance(amount).unwrap();
        // Initially the balance was reserved so now it is unreserved and then transfer is made
        pallet_generic_asset::Module::<T>::unreserve(&asset_id, from, amount_balance);
        let result = pallet_generic_asset::Module::<T>::make_transfer(&asset_id, from, to,
                                                                      amount_balance);
        // TODO: Make sure the result is ok
    }

    // Transfers the balance of traders
    fn transfer_asset_market(asset_id: T::AssetId, amount: FixedU128, from: &T::AccountId, to: &T::AssetId) {
        let amount_balance = Self::convert_fixed_u128_to_balance(amount).unwrap();
        let result = pallet_generic_asset::Module::<T>::make_transfer(&asset_id, from, to,
                                                                      amount_balance);
        // TODO: Make sure the result is ok
    }

    // Checks all the basic checks
    fn basic_order_checks(order: &Order<T>) -> Result<Orderbook<T>, Error<T>> {
        match order.order_type {
            OrderType::BidLimit | OrderType::AskLimit => {
                if order.price <= FixedU128::from(0) || order.quantity <= FixedU128::from(0) {
                    return Err(<Error<T>>::InvalidPriceOrQuantityLimit.into());
                }
            }
            OrderType::BidMarket => {
                if order.price <= FixedU128::from(0) {
                    return Err(<Error<T>>::InvalidBidMarketPrice.into());
                }
            }
            OrderType::AskMarket => {
                if order.quantity <= FixedU128::from(0) {
                    return Err(<Error<T>>::InvalidAskMarketQuantity.into());
                }
            }
        }
        if !<Orderbooks<T>>::contains_key(&order.trading_pair) {
            return Err(<Error<T>>::InvalidTradingPair.into());
        }
        let orderbook: Orderbook<T> = <Orderbooks<T>>::get(&order.trading_pair);
        match order.order_type {
            OrderType::BidLimit => {
                let base_balance = pallet_generic_asset::Module::<T>::free_balance(
                    &orderbook.base_asset_id, &order.trader);
                if let Some(base_balance_converted) = Self::convert_balance_to_fixed_u128(base_balance) {
                    let trade_amount = order.price.checked_mul(&order.quantity).unwrap(); // TODO: This is bad!!
                    if base_balance_converted >= trade_amount {
                        // TODO: Remove the unwraps
                        pallet_generic_asset::Module::<T>::reserve(
                            &orderbook.base_asset_id, &order.trader,
                            Self::convert_fixed_u128_to_balance(trade_amount).unwrap());
                        Ok(orderbook)
                    } else {
                        Err(<Error<T>>::InsufficientAssetBalance.into())
                    }
                } else {
                    Err(<Error<T>>::InternalErrorU128Balance.into())
                }
            }
            OrderType::BidMarket => {
                let base_balance = pallet_generic_asset::Module::<T>::free_balance(
                    &orderbook.base_asset_id, &order.trader);
                if let Some(base_balance_converted) = Self::convert_balance_to_fixed_u128(base_balance) {
                    if base_balance_converted >= order.price {
                        Ok(orderbook)
                    } else {
                        Err(<Error<T>>::InsufficientAssetBalance.into())
                    }
                } else {
                    Err(<Error<T>>::InternalErrorU128Balance.into())
                }
            }
            OrderType::AskMarket | OrderType::AskLimit => {
                let quote_balance = pallet_generic_asset::Module::<T>::free_balance(
                    &orderbook.quote_asset_id, &order.trader);
                if let Some(quote_balance_converted) = Self::convert_balance_to_fixed_u128(quote_balance) {
                    if quote_balance_converted >= order.quantity {
                        if order.order_type == OrderType::AskLimit {
                            // TODO: Remove the unwraps
                            pallet_generic_asset::Module::<T>::reserve(
                                &orderbook.quote_asset_id, &order.trader,
                                Self::convert_fixed_u128_to_balance(order.quantity).unwrap());
                            // Note Market Order's don't reserve users balance.
                        }
                        Ok(orderbook)
                    } else {
                        Err(<Error<T>>::InsufficientAssetBalance.into())
                    }
                } else {
                    Err(<Error<T>>::InternalErrorU128Balance.into())
                }
            }
        }
    }

    // Converts Balance to FixedU128 representation
    pub fn convert_balance_to_fixed_u128(x: T::Balance) -> Option<FixedU128> {
        if let Some(y) = TryInto::<u128>::try_into(x).ok() {
            FixedU128::from(y).checked_div(&FixedU128::from(1_000_000_000_000))
        } else {
            None
        }
    }

    // Converts FixedU128 to Balance representation
    pub fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<T::Balance> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(UniqueSaturatedFrom::<u128>::unique_saturated_from(balance_in_u128))
        } else {
            None
        }
    }
}