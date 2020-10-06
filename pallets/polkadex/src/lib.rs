#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, ensure};
use frame_support::traits::Get;
use frame_system::ensure_signed;
use pallet_generic_asset::AssetIdProvider;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_arithmetic::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, UniqueSaturatedFrom};
use sp_runtime::traits::Hash;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::convert::TryInto;
use sp_std::str;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;


pub trait Trait: frame_system::Trait + pallet_generic_asset::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type TradingPairReservationFee: Get<<Self as pallet_generic_asset::Trait>::Balance>;
}

decl_event!(
	pub enum Event<T> where Hash = <T as frame_system::Trait>::Hash,
	                        AccountId = <T as frame_system::Trait>::AccountId{
		/// New Trading pair is created [TradingPairHash]
		TradingPairCreated(Hash),
		/// New Limit Order Created [OrderId,TradingPairID,OrderType,Price,Quantity,Trader]
		NewLimitOrder(Hash,Hash,OrderType,FixedU128,FixedU128,AccountId),
		/// Market Order - Unfilled [OrderId,TradingPairID,OrderType,Price,Quantity,Trader]
		UnfilledMarketOrder(Hash,Hash,OrderType,FixedU128,FixedU128,AccountId),
		/// Market Order - Filled [OrderId,TradingPairID,OrderType,Price,Quantity,Trader]
		FilledMarketOrder(Hash,Hash,OrderType,FixedU128,FixedU128,AccountId),
		/// Limit Order Fulfilled  [OrderId,TradingPairID,OrderType,Price,Quantity,Trader]
		FulfilledLimitOrder(Hash,Hash,OrderType,FixedU128,FixedU128,AccountId),
		/// Limit Order Partial Fill  [OrderId,TradingPairID,OrderType,Price,Quantity,Trader]
		PartialFillLimitOrder(Hash,Hash,OrderType,FixedU128,FixedU128,AccountId),
	}
);

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
		/// Element not found
		NoElementFound,
		///Underflow or Overflow because of checkedMul
		MulUnderflowOrOverflow,
		///Underflow or Overflow because of checkedDiv
		DivUnderflowOrOverflow,
		///Underflow or Overflow because of checkedAdd
		AddUnderflowOrOverflow,
		///Underflow or Overflow because of checkedSub
		SubUnderflowOrOverflow,
		///Error generated during asset transfer
		ErrorWhileTransferingAsset,
		///Failed to reserve amount
		ReserveAmountFailed,
		/// Invalid Origin
		InvalidOrigin,
		/// Price doesn't match with active order's price
		CancelPriceDoesntMatch,
		/// TradingPair mismatch
		TradingPairMismatch,
		/// Invalid OrderID
		InvalidOrderID,
		///Price or Quantity too low
		PriceOrQuantityTooLow,
	}
}


decl_storage! {

	trait Store for Module<T: Trait> as DEXModule {

	/// Stores all the different price levels for all the trading pairs in a DoubleMap.
	PriceLevels get(fn get_pricelevels): double_map hasher(identity) T::Hash, hasher(blake2_128_concat) FixedU128 => LinkedPriceLevel<T>;

	/// Stores all the different active ask and bid levels in the system as a sorted vector mapped to it's TradingPair.
	AsksLevels get(fn get_askslevels): map hasher(identity) T::Hash => Vec<FixedU128>;
	BidsLevels get(fn get_bidslevels): map hasher(identity) T::Hash => Vec<FixedU128>;

	/// Stores the Orderbook struct for all available trading pairs.
	Orderbooks get(fn get_orderbooks): map hasher(identity) T::Hash => Orderbook<T>;

	/// Store MarketData of TradingPairs
	MarketInfo get(fn get_marketdata): double_map hasher(identity) T::Hash, hasher(blake2_128_concat) T::BlockNumber => MarketData;
	Nonce: u128;
	}
}




decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

        /// This method registers new Trading Pair in the system.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `quote_asset_id` - pallet_generic_asset AssetId of Counter Asset .
        ///
        /// * `base_asset_id` - pallet_generic_asset AssetId Base Asset.
        ///
        /// # Return
        ///
        ///  This function returns a status that, new Trading Pair is successfully registered or not.

		#[weight = 10000]
		pub fn register_new_orderbook(origin, quote_asset_id: u32, base_asset_id: u32) -> dispatch::DispatchResultWithPostInfo{
		    let trader = ensure_signed(origin)?;


		    ensure!(!(&quote_asset_id == &base_asset_id), <Error<T>>::SameAssetIdsError);

		    // Checks the tradingPair whether exists
		    let trading_pair_id = Self::create_trading_pair_id(&quote_asset_id,&base_asset_id);
		    ensure!(!<Orderbooks<T>>::contains_key(&trading_pair_id), <Error<T>>::TradingPairIDExists);
		    // BTC/ETH and ETH/BTC are considered the same market
		    let trading_pair_id_rev =  Self::create_trading_pair_id(&base_asset_id,&quote_asset_id);
		    ensure!(!<Orderbooks<T>>::contains_key(&trading_pair_id_rev), <Error<T>>::TradingPairIDExists);

		    // The origin should reserve a certain amount of SpendingAssetCurrency for registering the pair
		    ensure!(Self::reserve_balance_registration(&trader), <Error<T>>::InsufficientAssetBalance);
		    Self::create_order_book(quote_asset_id.into(),base_asset_id.into(),&trading_pair_id);
		    Self::deposit_event(RawEvent::TradingPairCreated(trading_pair_id));
		    Ok(Some(0).into())
	    }


        /// This method submits a new order in the System.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `order_type` - Type of Order. (BidLimit, BidMarket, AskLimit, AskMarket)
        ///
        /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
        ///
        /// * `price` - Price provided by Trader in base_asset.
        ///
        /// * `quantity` - Quantity provided by Trader in quote_asset.
        ///
        /// # Return
        ///
        ///  This function returns a status that, new Order is successfully created or not.
        #[weight = 10000]
	    pub fn submit_order(origin, order_type: OrderType, trading_pair: T::Hash, price: T::Balance, quantity: T::Balance) -> dispatch::DispatchResultWithPostInfo{
	        let trader = ensure_signed(origin)?;

            // TODO: Add a upper bound

            ensure!(price > 1000000.into() || quantity > 1000000.into(), <Error<T>>::PriceOrQuantityTooLow);
            let converted_price = Self::convert_balance_to_fixed_u128(price).ok_or(<Error<T>>::InternalErrorU128Balance)?;

            let converted_quantity = Self::convert_balance_to_fixed_u128(quantity).ok_or(<Error<T>>::InternalErrorU128Balance)?;
	        Self::execute_order(trader, order_type, trading_pair, converted_price, converted_quantity)?; // TODO: It maybe an error in which case take the fees else refund
	        Ok(Some(0).into())
	    }


        /// This method cancels a order in the System.
        /// # Arguments
        ///
        /// * `origin` - This contains the detail of Origin from where Transaction originated.
        ///
        /// * `order_type` - Type of Order. (BidLimit, BidMarket, AskLimit, AskMarket)
        ///
        /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
        ///
        /// * `price` - Price provided by Trader in base_asset.
        ///
        /// # Return
        ///
        ///  This function returns a status that, given Order is successfully canceled or not.
	    #[weight = 10000]
	    pub fn cancel_order(origin, order_id: T::Hash, trading_pair: T::Hash, price: T::Balance) -> dispatch::DispatchResultWithPostInfo {
	        let trader = ensure_signed(origin)?;

	        ensure!(<Orderbooks<T>>::contains_key(&trading_pair), <Error<T>>::InvalidTradingPair);
	        let converted_price = Self::convert_balance_to_fixed_u128(price).ok_or(<Error<T>>::InternalErrorU128Balance)?;
	        Self::cancel_order_from_orderbook(trader,order_id,trading_pair,converted_price)?;
	        Ok(Some(0).into())
	    }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderType {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ErrorRpc {
    IdMustBe32Byte,
    Fixedu128tou128conversionFailed,
    AssetIdConversionFailed,
    NoElementFound,
    ServerErrorWhileCallingAPI,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderTypeRPC {
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

impl<T> Order<T> where T: Trait {
    pub fn convert(self) -> Result<Order4RPC, ErrorRpc> {
        let order = Order4RPC {
            id: Self::account_to_bytes(&self.id)?,
            trading_pair: Self::account_to_bytes(&self.trading_pair)?,
            trader: Self::account_to_bytes(&self.trader)?,
            price: Self::convert_fixed_u128_to_balance(self.price).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            quantity: Self::convert_fixed_u128_to_balance(self.quantity).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            order_type: self.order_type,
        };
        Ok(order)
    }

    fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], ErrorRpc>
        where AccountId: Encode,
    {
        let account_vec = account.encode();
        ensure!(account_vec.len() == 32, ErrorRpc::IdMustBe32Byte);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_vec);
        Ok(bytes)
    }

    pub fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
}

#[derive(Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Order4RPC {
    id: [u8; 32],
    trading_pair: [u8; 32],
    trader: [u8; 32],
    price: Vec<u8>,
    quantity: Vec<u8>,
    order_type: OrderType,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct LinkedPriceLevel<T> where T: Trait {
    next: Option<FixedU128>,
    prev: Option<FixedU128>,
    orders: VecDeque<Order<T>>,
}

impl<T> LinkedPriceLevel<T> where T: Trait {
    fn convert(self) -> Result<LinkedPriceLevelRpc, ErrorRpc> {
        let linked_pirce_level = LinkedPriceLevelRpc {
            next: Self::convert_fixed_u128_to_balance(self.next.ok_or(ErrorRpc::NoElementFound)?).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            prev: Self::convert_fixed_u128_to_balance(self.prev.ok_or(ErrorRpc::NoElementFound)?).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            orders: Self::cov_de_vec(self.clone().orders)?,
        };
        Ok(linked_pirce_level)
    }

    fn cov_de_vec(temp: VecDeque<Order<T>>) -> Result<Vec<Order4RPC>, ErrorRpc> {
        let mut temp3: Vec<Order4RPC> = Vec::new();
        for element in temp {
            temp3.push(element.convert()?)
        };
        Ok(temp3)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();

            let hex_vec: Vec<u8> = balance_in_u128.encode();
            Some(hex_vec)
        } else {
            None
        }
    }
}

#[derive(Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LinkedPriceLevelRpc {
    next: Vec<u8>,
    prev: Vec<u8>,
    orders: Vec<Order4RPC>,
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct Orderbook<T> where T: Trait {
    trading_pair: T::Hash,
    base_asset_id: T::AssetId,
    quote_asset_id: T::AssetId,
    best_bid_price: FixedU128,
    best_ask_price: FixedU128,
}

impl<T> Orderbook<T> where T: Trait {
    fn convert(self) -> Result<OrderbookRpc, ErrorRpc> {
        let orderbook = OrderbookRpc {
            trading_pair: Self::account_to_bytes(&self.trading_pair)?,
            base_asset_id: TryInto::<u32>::try_into(self.base_asset_id).ok().ok_or(ErrorRpc::AssetIdConversionFailed)?,
            quote_asset_id: TryInto::<u32>::try_into(self.quote_asset_id).ok().ok_or(ErrorRpc::AssetIdConversionFailed)?,
            best_bid_price: Self::convert_fixed_u128_to_balance(self.best_bid_price).ok_or(ErrorRpc::IdMustBe32Byte)?,
            best_ask_price: Self::convert_fixed_u128_to_balance(self.best_ask_price).ok_or(ErrorRpc::IdMustBe32Byte)?,
        };
        Ok(orderbook)
    }

    fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], ErrorRpc>
        where AccountId: Encode,
    {
        let account_vec = account.encode();
        ensure!(account_vec.len() == 32, ErrorRpc::IdMustBe32Byte);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_vec);
        Ok(bytes)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
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

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OrderbookRpc {
    trading_pair: [u8; 32],
    base_asset_id: u32,
    quote_asset_id: u32,
    best_bid_price: Vec<u8>,
    best_ask_price: Vec<u8>,
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

impl MarketData {
    fn convert(self) -> Result<MarketDataRpc, ErrorRpc> {
        let market_data = MarketDataRpc {
            low: Self::convert_fixed_u128_to_balance(self.low).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            high: Self::convert_fixed_u128_to_balance(self.high).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            volume: Self::convert_fixed_u128_to_balance(self.volume).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
        };
        Ok(market_data)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
}

impl Default for MarketData {
    fn default() -> Self {
        MarketData {
            low: FixedU128::from(0),
            high: FixedU128::from(0),
            volume: FixedU128::from(0),
        }
    }
}

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MarketDataRpc {
    low: Vec<u8>,
    high: Vec<u8>,
    volume: Vec<u8>,
}

impl<T: Trait> Module<T> {
    /// This is a helper function for "Get Ask Level API".
    /// # Arguments
    ///
    /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
    ///
    /// # Return
    ///
    ///  This function returns List of Ask Level otherwise Related Error.

    pub fn get_ask_level(trading_pair: T::Hash) -> Result<Vec<FixedU128>, ErrorRpc> {
        let ask_level = <AsksLevels<T>>::get(trading_pair);
        Ok(ask_level)
    }

    /// This is a helper function for "Get Bid Level API".
    ///
    /// # Arguments
    ///
    /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
    ///
    /// # Return
    ///
    ///  This function returns List of Bid Level otherwise Related Error.
    pub fn get_bid_level(trading_pair: T::Hash) -> Result<Vec<FixedU128>, ErrorRpc> {
        let bid_level = <BidsLevels<T>>::get(trading_pair);
        Ok(bid_level)
    }

    /// This is a helper function for "Get Price Level API".
    ///
    /// # Arguments
    ///
    /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
    ///
    /// # Return
    ///
    ///  This function returns List of Price Level otherwise Related Error.
    pub fn get_price_level(trading_pair: T::Hash) -> Result<Vec<LinkedPriceLevelRpc>, ErrorRpc> {
        let price_level: Vec<LinkedPriceLevel<T>> = <PriceLevels<T>>::iter_prefix_values(&trading_pair).collect();
        //let temp2: Vec<LinkedPriceLevelRpc> = temp.into_iter().map(|element| element.covert()).collect();
        let mut price_level_rpc: Vec<LinkedPriceLevelRpc> = Vec::new();
        for element in price_level {
            price_level_rpc.push(element.convert()?)
        }
        Ok(price_level_rpc)
    }

    /// This is a helper function for "Get Orderbook API".
    ///
    /// # Arguments
    ///
    /// * `trading_pair` - Id of Trading Pair (quote_asset/base_asset).
    ///
    /// # Return
    ///
    ///  This function returns Requested Orderbook otherwise Related Error.
    pub fn get_orderbook(trading_pair: T::Hash) -> Result<OrderbookRpc, ErrorRpc> {
        let orderbook = <Orderbooks<T>>::get(trading_pair);
        let orderbook_rpc = orderbook.convert()?;
        Ok(orderbook_rpc)
    }

    /// This is a helper function for "Get All Orderbook API".
    ///
    /// # Arguments
    ///
    /// # Return
    ///
    ///  This function returns all Orderbooks otherwise Related Error.
    pub fn get_all_orderbook() -> Result<Vec<OrderbookRpc>, ErrorRpc> {
        let orderbook: Vec<Orderbook<T>> = <Orderbooks<T>>::iter().map(|(_key, value)| value).collect();
        let mut orderbook_rpc: Vec<OrderbookRpc> = Vec::new();
        for element in orderbook {
            orderbook_rpc.push(element.convert()?)
        }
        Ok(orderbook_rpc)
    }

    /// This is a helper function for "Get All Orderbook API".
    ///
    /// # Arguments
    ///
    /// # Return
    ///
    ///  This function returns all Orderbooks otherwise Related Error.

    pub fn get_market_info(trading_pair: T::Hash, blocknum: u32) -> Result<MarketDataRpc, ErrorRpc> {
        let blocknum = Self::u32_to_blocknum(blocknum);
        if <MarketInfo<T>>::contains_key(trading_pair, blocknum) {
            let temp: MarketData = <MarketInfo<T>>::get(trading_pair, blocknum);
            temp.convert()
        } else {
            Err(ErrorRpc::NoElementFound)
        }
    }

    pub fn u32_to_blocknum(input: u32) -> T::BlockNumber {
        input.into()
    }
}

impl<T: Trait> Module<T> {
    /// Reserves TradingPairReservationFee (defined in configuration trait) balance of SpendingAssetCurrency
    fn reserve_balance_registration(origin: &<T as frame_system::Trait>::AccountId) -> bool {
        pallet_generic_asset::Module::<T>::reserve(
            &pallet_generic_asset::SpendingAssetIdProvider::<T>::asset_id(),
            origin, <T as Trait>::TradingPairReservationFee::get()).is_ok()
    }

    /// Initializes a new Orderbook and stores it in the Orderbooks
    fn create_order_book(quote_asset_id: T::AssetId, base_asset_id: T::AssetId, trading_pair_id: &T::Hash) {
        let orderbook = Orderbook::new(base_asset_id, quote_asset_id, trading_pair_id.clone());
        <Orderbooks<T>>::insert(trading_pair_id, orderbook);
        <AsksLevels<T>>::insert(trading_pair_id, Vec::<FixedU128>::new());
        <BidsLevels<T>>::insert(trading_pair_id, Vec::<FixedU128>::new());
    }

    /// Creates a TradingPairID from both Asset IDs.
    fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> T::Hash {
        (quote_asset_id, base_asset_id).using_encoded(<T as frame_system::Trait>::Hashing::hash)
    }

    /// Submits an order for execution.
    fn execute_order(trader: T::AccountId,
                     order_type: OrderType,
                     trading_pair: T::Hash,
                     price: FixedU128,
                     quantity: FixedU128) -> Result<(), Error<T>> {
        let mut current_order = Order {
            id: T::Hash::default(),
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
                Nonce::put(nonce + 1);

                match current_order.order_type {
                    OrderType::AskMarket if orderbook.best_bid_price != FixedU128::from(0) => {
                        Self::consume_order(&mut current_order, &mut orderbook)?;
                    }

                    OrderType::BidMarket if orderbook.best_ask_price != FixedU128::from(0) => {
                        Self::consume_order(&mut current_order, &mut orderbook)?;
                    }

                    OrderType::AskLimit | OrderType::BidLimit => {
                        if (current_order.order_type == OrderType::BidLimit &&
                            current_order.price >= orderbook.best_ask_price &&
                            orderbook.best_ask_price != FixedU128::from(0)) ||
                            (current_order.order_type == OrderType::AskLimit &&
                                current_order.price <= orderbook.best_bid_price &&
                                orderbook.best_bid_price != FixedU128::from(0)) {
                            Self::consume_order(&mut current_order, &mut orderbook)?;


                            if current_order.quantity > FixedU128::from(0) {
                                Self::insert_order(&current_order, &mut orderbook)?;
                            }
                        } else {
                            Self::insert_order(&current_order, &mut orderbook)?;
                        }
                    }
                    _ => {}
                }
                <Orderbooks<T>>::insert(&current_order.trading_pair, orderbook);
                match current_order.order_type {
                    OrderType::BidLimit | OrderType::AskLimit if current_order.quantity > FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::NewLimitOrder(current_order.id,
                                                                    current_order.trading_pair,
                                                                    current_order.order_type,
                                                                    current_order.price,
                                                                    current_order.quantity,
                                                                    current_order.trader));
                    }
                    OrderType::BidMarket if current_order.price > FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::UnfilledMarketOrder(current_order.id,
                                                                          current_order.trading_pair,
                                                                          current_order.order_type,
                                                                          current_order.price,
                                                                          current_order.quantity,
                                                                          current_order.trader));
                    }
                    OrderType::AskMarket if current_order.quantity > FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::UnfilledMarketOrder(current_order.id,
                                                                          current_order.trading_pair,
                                                                          current_order.order_type,
                                                                          current_order.price,
                                                                          current_order.quantity,
                                                                          current_order.trader));
                    }
                    OrderType::BidLimit | OrderType::AskLimit if current_order.quantity == FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::FulfilledLimitOrder(current_order.id,
                                                                          current_order.trading_pair,
                                                                          current_order.order_type,
                                                                          current_order.price,
                                                                          current_order.quantity,
                                                                          current_order.trader));
                    }
                    OrderType::BidMarket if current_order.price == FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::FilledMarketOrder(current_order.id,
                                                                        current_order.trading_pair,
                                                                        current_order.order_type,
                                                                        current_order.price,
                                                                        current_order.quantity,
                                                                        current_order.trader));
                    }
                    OrderType::AskMarket if current_order.quantity == FixedU128::from(0) => {
                        Self::deposit_event(RawEvent::FilledMarketOrder(current_order.id,
                                                                        current_order.trading_pair,
                                                                        current_order.order_type,
                                                                        current_order.price,
                                                                        current_order.quantity,
                                                                        current_order.trader));
                    }
                    _ => {}
                }
                Ok(())
            }
            Err(err_value) => Err(err_value),
        }
    }

    /// Inserts the given order into orderbook.
    fn insert_order(current_order: &Order<T>, orderbook: &mut Orderbook<T>) -> Result<(), Error<T>> {
        match current_order.order_type {
            OrderType::BidLimit => {
                let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                match bids_levels.binary_search(&current_order.price) {
                    Ok(_) => {
                        let mut linked_pricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &current_order.price);
                        linked_pricelevel.orders.push_back(current_order.clone());

                        <PriceLevels<T>>::insert(&current_order.trading_pair, &current_order.price, linked_pricelevel)
                    }
                    Err(index_at_which_we_should_insert) => {
                        bids_levels.insert(index_at_which_we_should_insert, current_order.price);

                        if index_at_which_we_should_insert != 0 && index_at_which_we_should_insert != bids_levels.len() - 1 {
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?),
                                prev: Some(*bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.prev = Some(current_order.price);
                            index_plus1_linkedpricelevel.next = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());


                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }

                        if index_at_which_we_should_insert == 0 && bids_levels.len() > 1 {
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: Some(*bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_plus1_linkedpricelevel.next = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        } else if index_at_which_we_should_insert == 0 && bids_levels.len() == 1 {
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            current_linkedpricelevel.orders.push_back(current_order.clone());
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);


                            orderbook.best_bid_price = current_order.price;
                        }
                        if index_at_which_we_should_insert == bids_levels.len() - 1 && index_at_which_we_should_insert != 0 {
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?),
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.prev = Some(current_order.price);

                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &bids_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);


                            orderbook.best_bid_price = current_order.price;
                        }
                    }
                }
                <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);
            }
            OrderType::AskLimit => {
                let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                match asks_levels.binary_search(&current_order.price) {
                    Ok(_) => {
                        let mut linked_pricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &current_order.price);
                        linked_pricelevel.orders.push_back(current_order.clone());

                        <PriceLevels<T>>::insert(&current_order.trading_pair, &current_order.price, linked_pricelevel)
                    }
                    Err(index_at_which_we_should_insert) => {
                        asks_levels.insert(index_at_which_we_should_insert, current_order.price);

                        if index_at_which_we_should_insert != 0 && index_at_which_we_should_insert != asks_levels.len() - 1 {
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?),
                                prev: Some(*asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());


                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }

                        if index_at_which_we_should_insert == 0 && asks_levels.len() > 1 {
                            let mut index_plus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: Some(*asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?),
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_plus1_linkedpricelevel.prev = Some(current_order.price);

                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert + 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_plus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);


                            orderbook.best_ask_price = current_order.price;
                        }
                        if index_at_which_we_should_insert == 0 && asks_levels.len() == 1 {
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: None,
                                orders: VecDeque::<Order<T>>::new(),
                            };

                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);

                            orderbook.best_ask_price = current_order.price;
                        }
                        if index_at_which_we_should_insert == asks_levels.len() - 1 && index_at_which_we_should_insert != 0 {
                            let mut index_minus1_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&current_order.trading_pair, &asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?);
                            let mut current_linkedpricelevel: LinkedPriceLevel<T> = LinkedPriceLevel {
                                next: None,
                                prev: Some(*asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?),
                                orders: VecDeque::<Order<T>>::new(),
                            };
                            index_minus1_linkedpricelevel.next = Some(current_order.price);
                            current_linkedpricelevel.orders.push_back(current_order.clone());

                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &asks_levels.get(index_at_which_we_should_insert - 1).ok_or(Error::<T>::NoElementFound.into())?,
                                                     index_minus1_linkedpricelevel);
                            <PriceLevels<T>>::insert(&current_order.trading_pair,
                                                     &current_order.price,
                                                     current_linkedpricelevel);
                        }
                    }
                }
                <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);
            }
            _ => {}
        }
        Ok(())
    }
    /// The incoming order is matched against existing orders from orderbook
    fn consume_order(current_order: &mut Order<T>, orderbook: &mut Orderbook<T>) -> Result<(), Error<T>> {
        let mut market_data: MarketData;

        let current_block_number: T::BlockNumber = <frame_system::Module<T>>::block_number();
        if <MarketInfo<T>>::contains_key(&current_order.trading_pair, current_block_number) {
            market_data = <MarketInfo<T>>::get(&current_order.trading_pair, <frame_system::Module<T>>::block_number())
        } else {
            market_data = MarketData {
                low: FixedU128::from(0),
                high: FixedU128::from(0),
                volume: FixedU128::from(0),
            }
        }

        match current_order.order_type {
            OrderType::BidLimit => {
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_ask_price);
                let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                while current_order.quantity > FixedU128::from(0) {
                    if let Some(mut counter_order) = linkedpricelevel.orders.pop_front() {
                        Self::do_asset_exchange(current_order,
                                                &mut counter_order,
                                                &mut market_data,
                                                orderbook.base_asset_id,
                                                orderbook.quote_asset_id)?;

                        if counter_order.quantity > FixedU128::from(0) {
                            Self::emit_partial_fill(&counter_order, current_order.quantity);

                            linkedpricelevel.orders.push_front(counter_order);
                        } else {
                            Self::emit_complete_fill(&counter_order, current_order.quantity);
                        }
                    } else {
                        asks_levels.remove(0);


                        if linkedpricelevel.next.is_none() {
                            break;
                        }

                        if current_order.price >= linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())? {
                            orderbook.best_ask_price = linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?;
                            linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?);
                        } else {
                            break;
                        }
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_ask_price, linkedpricelevel);
                } else {
                    asks_levels.remove(0);

                    if asks_levels.len() == 0 {
                        orderbook.best_ask_price = FixedU128::from(0);
                    } else {
                        match asks_levels.get(0) {
                            Some(best_price) => {
                                orderbook.best_ask_price = *best_price;
                            }
                            None => {
                                orderbook.best_ask_price = FixedU128::from(0);
                            }
                        }
                    }
                }


                <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);
            }

            OrderType::BidMarket => {
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_ask_price);
                let mut asks_levels: Vec<FixedU128> = <AsksLevels<T>>::get(&current_order.trading_pair);
                while current_order.price > FixedU128::from(0) {
                    if let Some(mut counter_order) = linkedpricelevel.orders.pop_front() {
                        Self::do_asset_exchange_market(current_order,
                                                       &mut counter_order,
                                                       &mut market_data,
                                                       orderbook.base_asset_id,
                                                       orderbook.quote_asset_id)?;


                        if counter_order.quantity > FixedU128::from(0) {
                            // Emit events
                            Self::emit_partial_fill(&counter_order, current_order.quantity);
                            // counter_order was not completely used so we store it back in the FIFO
                            linkedpricelevel.orders.push_front(counter_order);
                        } else {
                            // Emit events
                            Self::emit_complete_fill(&counter_order, current_order.quantity);
                        }
                    } else {
                        asks_levels.remove(0);


                        if linkedpricelevel.next.is_none() {
                            break;
                        }

                        orderbook.best_ask_price = linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?;
                        linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?);
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_ask_price, linkedpricelevel);
                } else {
                    asks_levels.remove(0);

                    if asks_levels.len() == 0 {
                        orderbook.best_ask_price = FixedU128::from(0);
                    } else {
                        match asks_levels.get(0) {
                            Some(best_price) => {
                                orderbook.best_ask_price = *best_price;
                            }
                            None => {
                                orderbook.best_ask_price = FixedU128::from(0);
                            }
                        }
                    }
                }


                <AsksLevels<T>>::insert(&current_order.trading_pair, asks_levels);
            }

            OrderType::AskLimit => {
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_bid_price);
                let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                while current_order.quantity > FixedU128::from(0) {
                    if let Some(mut counter_order) = linkedpricelevel.orders.pop_front() {
                        Self::do_asset_exchange(current_order,
                                                &mut counter_order,
                                                &mut market_data,
                                                orderbook.base_asset_id,
                                                orderbook.quote_asset_id)?;

                        if counter_order.quantity > FixedU128::from(0) {
                            Self::emit_partial_fill(&counter_order, current_order.quantity);

                            linkedpricelevel.orders.push_front(counter_order);
                        } else {
                            Self::emit_complete_fill(&counter_order, current_order.quantity);
                        }
                    } else {
                        bids_levels.remove(bids_levels.len() - 1);


                        if linkedpricelevel.next.is_none() {
                            break;
                        }
                        if current_order.price <= linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())? {
                            orderbook.best_bid_price = linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?;
                            linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?);
                        } else {
                            break;
                        }
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_bid_price, linkedpricelevel);
                } else {
                    if bids_levels.len() != 0 {
                        bids_levels.remove(bids_levels.len() - 1);
                    }

                    if bids_levels.len() == 0 {
                        orderbook.best_bid_price = FixedU128::from(0);
                    } else {
                        match bids_levels.get(bids_levels.len() - 1) {
                            Some(best_price) => {
                                orderbook.best_bid_price = *best_price;
                            }
                            None => {
                                orderbook.best_bid_price = FixedU128::from(0);
                            }
                        }
                    }
                }


                <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);
            }

            OrderType::AskMarket => {
                let mut linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(&current_order.trading_pair, &orderbook.best_bid_price);
                let mut bids_levels: Vec<FixedU128> = <BidsLevels<T>>::get(&current_order.trading_pair);
                while current_order.quantity > FixedU128::from(0) {
                    if let Some(mut counter_order) = linkedpricelevel.orders.pop_front() {
                        Self::do_asset_exchange_market(current_order,
                                                       &mut counter_order,
                                                       &mut market_data,
                                                       orderbook.base_asset_id,
                                                       orderbook.quote_asset_id)?;

                        if counter_order.quantity > FixedU128::from(0) {
                            Self::emit_partial_fill(&counter_order, current_order.quantity);

                            linkedpricelevel.orders.push_front(counter_order);
                        } else {
                            Self::emit_complete_fill(&counter_order, current_order.quantity);
                        }
                    } else {
                        bids_levels.remove(bids_levels.len() - 1);

                        if linkedpricelevel.next.is_none() {
                            break;
                        }

                        orderbook.best_bid_price = linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?;
                        linkedpricelevel = <PriceLevels<T>>::take(&current_order.trading_pair, linkedpricelevel.next.ok_or(Error::<T>::NoElementFound.into())?);
                    }
                }

                if !linkedpricelevel.orders.is_empty() {
                    <PriceLevels<T>>::insert(&current_order.trading_pair, &orderbook.best_bid_price, linkedpricelevel);
                } else {
                    if bids_levels.len() != 0 {
                        bids_levels.remove(bids_levels.len() - 1);
                    }

                    if bids_levels.len() == 0 {
                        orderbook.best_bid_price = FixedU128::from(0);
                    } else {
                        match bids_levels.get(bids_levels.len() - 1) {
                            Some(best_price) => {
                                orderbook.best_bid_price = *best_price;
                            }
                            None => {
                                orderbook.best_bid_price = FixedU128::from(0);
                            }
                        }
                    }
                }


                <BidsLevels<T>>::insert(&current_order.trading_pair, bids_levels);
            }
        }

        <MarketInfo<T>>::insert(&current_order.trading_pair, current_block_number, market_data);
        Ok(())
    }
    /// Function un-reserves and transfers assets balances between traders
    fn do_asset_exchange_market(current_order: &mut Order<T>, counter_order: &mut Order<T>, market_data: &mut MarketData, base_assetid: T::AssetId, quote_assetid: T::AssetId) -> Result<(), Error<T>> {
        if market_data.low == FixedU128::from(0) {
            market_data.low = counter_order.price
        }
        if market_data.high == FixedU128::from(0) {
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
                let current_order_quantity = current_order.price.checked_div(&counter_order.price).ok_or(Error::<T>::DivUnderflowOrOverflow.into())?;

                if current_order_quantity <= counter_order.quantity {
                    Self::transfer_asset_market(base_assetid, current_order.price, &current_order.trader, &counter_order.trader)?;

                    Self::transfer_asset(quote_assetid, current_order_quantity, &counter_order.trader, &current_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&current_order.price).ok_or(Error::<T>::AddUnderflowOrOverflow.into())?;

                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order_quantity).ok_or(Error::<T>::SubUnderflowOrOverflow.into())?;
                    current_order.price = FixedU128::from(0);
                } else {
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).ok_or(Error::<T>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset_market(base_assetid, trade_amount, &current_order.trader, &counter_order.trader)?;

                    Self::transfer_asset(quote_assetid, counter_order.quantity, &counter_order.trader, &current_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(Error::<T>::AddUnderflowOrOverflow.into())?;

                    counter_order.quantity = FixedU128::from(0);
                    current_order.price = current_order.price.checked_sub(&trade_amount).ok_or(Error::<T>::SubUnderflowOrOverflow.into())?;
                }
            }
            OrderType::AskMarket => {
                if current_order.quantity <= counter_order.quantity {
                    let trade_amount = counter_order.price.checked_mul(&current_order.quantity).ok_or(Error::<T>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader)?;

                    Self::transfer_asset_market(quote_assetid, current_order.quantity, &current_order.trader, &counter_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(Error::<T>::AddUnderflowOrOverflow.into())?;

                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).ok_or(Error::<T>::SubUnderflowOrOverflow.into())?;
                    current_order.quantity = FixedU128::from(0);
                } else {
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).ok_or(Error::<T>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader)?;

                    Self::transfer_asset_market(quote_assetid, counter_order.quantity, &current_order.trader, &counter_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(Error::<T>::AddUnderflowOrOverflow.into())?;

                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).ok_or(Error::<T>::SubUnderflowOrOverflow.into())?;
                    counter_order.quantity = FixedU128::from(0);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Function un-reserves and transfers assets balances between traders
    fn do_asset_exchange(current_order: &mut Order<T>, counter_order: &mut Order<T>, market_data: &mut MarketData, base_assetid: T::AssetId, quote_assetid: T::AssetId) -> Result<(), Error<T>> {
        if market_data.low == FixedU128::from(0) {
            market_data.low = counter_order.price
        }
        if market_data.high == FixedU128::from(0) {
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
                if current_order.quantity <= counter_order.quantity {
                    let trade_amount = current_order.price.checked_mul(&current_order.quantity).ok_or(<Error<T>>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &current_order.trader, &counter_order.trader)?;

                    Self::transfer_asset(quote_assetid, current_order.quantity, &counter_order.trader, &current_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(<Error<T>>::AddUnderflowOrOverflow.into())?;

                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;
                    current_order.quantity = FixedU128::from(0);
                } else {
                    let trade_amount = current_order.price.checked_mul(&counter_order.quantity).ok_or(<Error<T>>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &current_order.trader, &counter_order.trader)?;

                    Self::transfer_asset(quote_assetid, counter_order.quantity, &counter_order.trader, &current_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(<Error<T>>::MulUnderflowOrOverflow.into())?;
                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;
                    counter_order.quantity = FixedU128::from(0);
                }
            }
            OrderType::AskLimit => {
                if current_order.quantity <= counter_order.quantity {
                    let trade_amount = counter_order.price.checked_mul(&current_order.quantity).ok_or(<Error<T>>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader)?;

                    Self::transfer_asset(quote_assetid, current_order.quantity, &current_order.trader, &counter_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(<Error<T>>::AddUnderflowOrOverflow.into())?;

                    counter_order.quantity = counter_order.quantity.checked_sub(&current_order.quantity).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;
                    current_order.quantity = FixedU128::from(0);
                } else {
                    let trade_amount = counter_order.price.checked_mul(&counter_order.quantity).ok_or(<Error<T>>::MulUnderflowOrOverflow.into())?;

                    Self::transfer_asset(base_assetid, trade_amount, &counter_order.trader, &current_order.trader)?;

                    Self::transfer_asset(quote_assetid, counter_order.quantity, &current_order.trader, &counter_order.trader)?;

                    market_data.volume = market_data.volume.checked_add(&trade_amount).ok_or(<Error<T>>::AddUnderflowOrOverflow.into())?;
                    current_order.quantity = current_order.quantity.checked_sub(&counter_order.quantity).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;
                    counter_order.quantity = FixedU128::from(0);
                }
            }

            _ => {}
        }
        Ok(())
    }

    /// Transfers the balance of traders
    fn transfer_asset(asset_id: T::AssetId, amount: FixedU128, from: &T::AccountId, to: &T::AccountId) -> Result<(), Error<T>> {
        let amount_balance = Self::convert_fixed_u128_to_balance(amount).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;

        pallet_generic_asset::Module::<T>::unreserve(&asset_id, from, amount_balance);
        match pallet_generic_asset::Module::<T>::make_transfer(&asset_id, from, to,
                                                               amount_balance) {
            Ok(_) => Ok(()),
            _ => Err(<Error<T>>::ErrorWhileTransferingAsset.into()),
        }
    }

    /// Transfers the balance of traders
    fn transfer_asset_market(asset_id: T::AssetId, amount: FixedU128, from: &T::AccountId, to: &T::AccountId) -> Result<(), Error<T>> {
        let amount_balance = Self::convert_fixed_u128_to_balance(amount).ok_or(<Error<T>>::SubUnderflowOrOverflow.into())?;
        match pallet_generic_asset::Module::<T>::make_transfer(&asset_id, from, to,
                                                               amount_balance) {
            Ok(_) => Ok(()),
            _ => Err(<Error<T>>::ErrorWhileTransferingAsset.into()),
        }
    }

    /// Checks all the basic checks
    fn basic_order_checks(order: &Order<T>) -> Result<Orderbook<T>, Error<T>> {
        match order.order_type {
            OrderType::BidLimit | OrderType::AskLimit if order.price <= FixedU128::from(0) || order.quantity <= FixedU128::from(0) => Err(<Error<T>>::InvalidPriceOrQuantityLimit.into()),
            OrderType::BidMarket if order.price <= FixedU128::from(0) => Err(<Error<T>>::InvalidBidMarketPrice.into()),
            OrderType::BidMarket | OrderType::BidLimit => Self::check_order(order),
            OrderType::AskMarket if order.quantity <= FixedU128::from(0) => Err(<Error<T>>::InvalidAskMarketQuantity.into()),
            OrderType::AskMarket | OrderType::AskLimit => Self::check_order(order),
        }
    }
    /// Helper function for basic_order_check
    fn check_order(order: &Order<T>) -> Result<Orderbook<T>, Error<T>> {
        let orderbook: Orderbook<T> = <Orderbooks<T>>::get(&order.trading_pair);
        let balance: <T>::Balance = match order.order_type {
            OrderType::BidLimit | OrderType::BidMarket => pallet_generic_asset::Module::<T>::free_balance(&orderbook.base_asset_id, &order.trader),
            OrderType::AskMarket | OrderType::AskLimit => pallet_generic_asset::Module::<T>::free_balance(&orderbook.quote_asset_id, &order.trader),
        };

        match Self::convert_balance_to_fixed_u128(balance) {
            Some(converted_balance) if order.order_type == OrderType::BidLimit => Self::compare_balance(converted_balance, order, orderbook),
            Some(converted_balance) if order.order_type == OrderType::BidMarket && converted_balance < order.price => Err(<Error<T>>::InsufficientAssetBalance.into()),
            Some(converted_balance) if (order.order_type == OrderType::AskLimit || order.order_type == OrderType::AskMarket) && converted_balance < order.quantity => Err(<Error<T>>::InsufficientAssetBalance.into()),
            Some(_) if order.order_type == OrderType::AskLimit => Self::reserve_user_balance(orderbook, order, order.quantity),
            Some(_) if order.order_type == OrderType::AskMarket => Ok(orderbook),
            Some(_) if order.order_type == OrderType::BidMarket => Ok(orderbook),
            _ => Err(<Error<T>>::InternalErrorU128Balance.into()),
        }
    }
    /// Helper function for basic_order_check
    fn compare_balance(converted_balance: FixedU128, order: &Order<T>, orderbook: Orderbook<T>) -> Result<Orderbook<T>, Error<T>> {
        match order.price.checked_mul(&order.quantity) {
            Some(trade_amount) if converted_balance < trade_amount => Err(<Error<T>>::InsufficientAssetBalance.into()),
            Some(trade_amount) if converted_balance >= trade_amount => Self::reserve_user_balance(orderbook, order, trade_amount),
            _ => Err(<Error<T>>::InternalErrorU128Balance.into()),
        }
    }
    /// Helper function for basic_order_check
    fn reserve_user_balance(orderbook: Orderbook<T>, order: &Order<T>, amount: FixedU128) -> Result<Orderbook<T>, Error<T>> {
        // TODO: Based on BidLimit or AskLimit we need to change between orderbook.base_asset_id & orderbook.quote_asset_id respectively
        let asset_id = if order.order_type == OrderType::AskLimit { &orderbook.quote_asset_id } else { &orderbook.base_asset_id };

        match Self::convert_fixed_u128_to_balance(amount) {
            Some(balance) => {
                match pallet_generic_asset::Module::<T>::reserve(
                    asset_id, &order.trader,
                    balance) {
                    Ok(_) => Ok(orderbook),
                    _ => Err(<Error<T>>::ReserveAmountFailed.into()),
                }
            }

            None => Err(<Error<T>>::InternalErrorU128Balance.into()),
        }
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

    /// Function for emitting event.
    pub fn emit_partial_fill(order: &Order<T>, filled_amount: FixedU128) {
        Self::deposit_event(RawEvent::PartialFillLimitOrder(order.id,
                                                            order.trading_pair,
                                                            order.order_type.clone(),
                                                            order.price,
                                                            filled_amount,
                                                            order.trader.clone()));
    }

    /// Function for emitting event.
    pub fn emit_complete_fill(order: &Order<T>, filled_amount: FixedU128) {
        Self::deposit_event(RawEvent::FulfilledLimitOrder(order.id,
                                                          order.trading_pair,
                                                          order.order_type.clone(),
                                                          order.price,
                                                          filled_amount,
                                                          order.trader.clone()));
    }

    /// Cancels an existing active order
    pub fn cancel_order_from_orderbook(trader: T::AccountId, order_id: T::Hash, trading_pair: T::Hash, price: FixedU128) -> Result<(), Error<T>> {
        let mut current_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::take(trading_pair, price);
        let mut index = 0;
        let mut match_flag = false;

        let mut removed_order: Order<T> = Order {
            id: Default::default(),
            trading_pair: Default::default(),
            trader: Default::default(),
            price: Default::default(),
            quantity: Default::default(),
            order_type: OrderType::BidLimit,
        };

        for order in current_linkedpricelevel.orders.iter() {
            if order.id == order_id {
                removed_order = current_linkedpricelevel.orders.remove(index).ok_or(<Error<T>>::NoElementFound.into())?; // later
                match_flag = true;
                break;
            }
            index = index + 1;
        }
        ensure!(match_flag, <Error<T>>::InvalidOrderID);
        ensure!(removed_order.trader == trader,<Error<T>>::InvalidOrigin);
        ensure!(removed_order.trading_pair == trading_pair,<Error<T>>::TradingPairMismatch);
        ensure!(removed_order.price == price,<Error<T>>::CancelPriceDoesntMatch);


        if !current_linkedpricelevel.orders.is_empty() {
            <PriceLevels<T>>::insert(trading_pair, price, current_linkedpricelevel);
            return Ok(());
        }

        if current_linkedpricelevel.prev.is_some() && current_linkedpricelevel.next.is_some() {
            let mut prev_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(trading_pair, current_linkedpricelevel.prev.ok_or(<Error<T>>::NoElementFound.into())?); //later
            let mut next_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(trading_pair, current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?); //later


            prev_linkedpricelevel.next = current_linkedpricelevel.next;
            next_linkedpricelevel.prev = current_linkedpricelevel.prev;


            <PriceLevels<T>>::insert(trading_pair, current_linkedpricelevel.prev.ok_or(<Error<T>>::NoElementFound.into())?, prev_linkedpricelevel); //later
            <PriceLevels<T>>::insert(trading_pair, current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?, next_linkedpricelevel); //later
        }

        if current_linkedpricelevel.prev.is_some() && current_linkedpricelevel.next.is_none() {
            let mut prev_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(trading_pair, current_linkedpricelevel.prev.ok_or(<Error<T>>::NoElementFound.into())?); //later


            prev_linkedpricelevel.next = None;


            <PriceLevels<T>>::insert(trading_pair, current_linkedpricelevel.prev.ok_or(<Error<T>>::NoElementFound.into())?, prev_linkedpricelevel); //later
        }
        if current_linkedpricelevel.prev.is_none() && current_linkedpricelevel.next.is_some() {
            let mut next_linkedpricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(trading_pair, current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?); //later


            next_linkedpricelevel.prev = None;

            // Write it back
            <PriceLevels<T>>::insert(trading_pair, current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?, next_linkedpricelevel); //later


            let mut orderbook: Orderbook<T> = <Orderbooks<T>>::get(trading_pair);

            if removed_order.order_type == OrderType::BidLimit && price == orderbook.best_bid_price {
                orderbook.best_bid_price = current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?;   //later
            }

            if removed_order.order_type == OrderType::AskLimit && price == orderbook.best_ask_price {
                orderbook.best_ask_price = current_linkedpricelevel.next.ok_or(<Error<T>>::NoElementFound.into())?;  //later
            }

            <Orderbooks<T>>::insert(trading_pair, orderbook);
        }
        Ok(())
    }
}