#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch};
use frame_system::ensure_signed;
use codec::{Encode,Decode};
use sp_std::str;
use sp_std::vec::Vec;
use sp_arithmetic::FixedU128;
use sp_std::collections::vec_deque::VecDeque;

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
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, AccountId),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
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

	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OrderType{
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Order<T> where T: Trait {
    id: T::Hash,
    trading_pair: T::Hash,
    trader: T::AccountId,
    price: FixedU128,  // MarketBuy buy for 100 INR
    quantity: FixedU128, // MarketSell sell 5 quantities.
    order_type: OrderType
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct LinkedPriceLevel<T> where T: Trait {
    next: Option<FixedU128>,
    prev: Option<FixedU128>,
    orders: VecDeque<Order<T>>
}

impl<T> Default for LinkedPriceLevel<T> where T: Trait{
    fn default() -> Self {
        LinkedPriceLevel{
            next: None,
            prev: None,
            orders: Default::default()
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Orderbook<T> where T: Trait {
    base_asset_id: T::AssetId,
    quote_asset_id: T::AssetId,
    best_bid_price: FixedU128,
    best_ask_price: FixedU128
}
impl<T> Default for Orderbook<T> where T: Trait{
    fn default() -> Self {
        Orderbook{
            base_asset_id: 0.into(),
            quote_asset_id: 0.into(),
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0)
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MarketData{
    // Lowest price at which the trade was executed in a block.
    low: FixedU128,
    // Highest price at which the trade was executed in a block.
    high: FixedU128,
    // Total volume traded in a block.
    volume: FixedU128
}

impl<T: Trait> Module<T> {

}