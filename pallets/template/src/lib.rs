#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_support::traits::Get;
use frame_system::ensure_signed;
use pallet_generic_asset::AssetIdProvider;
use sp_arithmetic::FixedU128;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::str;
use sp_std::vec::Vec;
use sp_runtime::traits::Hash;

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
		    let _trader = ensure_signed(origin)?;

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
		    if Self::reserve_balance_registration(&_trader){
		        // Create the orderbook
		        Self::create_order_book(quote_asset_id.into(),base_asset_id.into(),&trading_pair_id);
		        Self::deposit_event(RawEvent::TradingPairCreated(trading_pair_id));
		        return Ok(Some(0).into());
		    }else{
		        return Err(<Error<T>>::InsufficientAssetBalance.into());
		    }
	    }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OrderType {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Order<T> where T: Trait {
    id: T::Hash,
    trading_pair: T::Hash,
    trader: T::AccountId,
    price: FixedU128,
    quantity: FixedU128,
    order_type: OrderType,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
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
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Orderbook<T> where T: Trait {
    base_asset_id: T::AssetId,
    quote_asset_id: T::AssetId,
    best_bid_price: FixedU128,
    best_ask_price: FixedU128,
}

impl<T> Default for Orderbook<T> where T: Trait {
    fn default() -> Self {
        Orderbook {
            base_asset_id: 0.into(),
            quote_asset_id: 0.into(),
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0),
        }
    }
}

impl<T> Orderbook<T> where T: Trait {
    fn new(base_asset_id: T::AssetId, quote_asset_id: T::AssetId) -> Self {

        Orderbook{
            base_asset_id,
            quote_asset_id,
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0)
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
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
    fn create_order_book(quote_asset_id: T::AssetId, base_asset_id: T::AssetId, trading_pair_id: &T::Hash){
        let orderbook = Orderbook::new(base_asset_id,quote_asset_id);
        <Orderbooks<T>>::insert(trading_pair_id, orderbook);
        <AsksLevels<T>>::insert(trading_pair_id,Vec::<FixedU128>::new());
        <BidsLevels<T>>::insert(trading_pair_id,Vec::<FixedU128>::new());
    }

    fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> T::Hash{
        (quote_asset_id,base_asset_id).using_encoded(
            <T as frame_system::Trait>::Hashing::hash)
    }
}