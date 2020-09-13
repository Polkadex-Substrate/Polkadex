use sp_arithmetic::FixedU128;
use codec::{Encode, Decode};
use sp_std::cmp::Ordering;
use sp_std::vec::Vec;
use sp_std::collections::vec_deque::VecDeque;
use crate::binary_heap::{BinaryHeap, MinComparator};
use sp_std::collections::btree_map;

pub type TradingPair = u32;

/// The available order types for the first version.
#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub enum OrderType {
    BidLimit,
    BidMarket,

    AskLimit,
    AskMarket,

    None,
}

impl Default for OrderType {
    fn default() -> Self { OrderType::None }
}

/// The representation of Order in this pallet
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Order<AccountId, BlockNumber> {
    pub(crate) id: Vec<u8>, // unique id that identifies a order
    pub(crate) order_type: OrderType, // order type
    pub(crate) price: FixedU128, // Fixed Point representation of order price
    pub(crate) quantity: FixedU128, // Fixed Point representation of order quantity
    pub(crate) market_maker: bool, // used for advanced not needed for Limit and Market
    pub(crate) origin: AccountId, // trader account which placed this order
    pub(crate) expiry: BlockNumber, // used for advanced not needed for Limit and Market
}

impl<AccountId, BlockNumber> Order<AccountId, BlockNumber> {
    pub fn get_origin(&self) -> &AccountId {
        return &self.origin;
    }

    pub fn get_quantity(&self) -> &FixedU128{
        return &self.quantity;
    }

    pub fn get_price(&self) -> &FixedU128{
        return &self.price;
    }

    pub fn get_expiry(&self) -> &BlockNumber{
        return &self.expiry;
    }

    pub fn get_id(&self) -> &Vec<u8>{
        return &self.id;
    }

    pub fn get_order_type(&self) -> &OrderType{
        return &self.order_type;
    }

    pub fn set_quantity(&mut self, new_quantity: FixedU128){
        self.quantity = new_quantity
    }
}

/// It is used to define the different price levels in the BinaryHeap.
/// All orders are grouped together based on their prices
/// queue: it contains a FIFO queue and new orders are to back of it and executed from front of the
/// queue. It gives a sense of price-time priority execution of trades.
#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct PriceLevel<AccountId, BlockNumber> {
    pub(crate) price_level: FixedU128,
    pub(crate) queue: VecDeque<Order<AccountId, BlockNumber>>,
}

impl<AccoundId,BlockNumber> PriceLevel<AccoundId,BlockNumber>{

    pub fn get_price_level(&self) -> &FixedU128 {
        &self.price_level
    }

    pub fn get_orders(&self) -> &VecDeque<Order<AccoundId, BlockNumber>> {
        &self.queue
    }

    pub fn get_orders_mut(&mut self) -> &mut VecDeque<Order<AccoundId, BlockNumber>>{
        &mut self.queue
    }
}

impl<AccountId, BlockNumber> Ord for PriceLevel<AccountId, BlockNumber> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price_level.cmp(&other.price_level)
    }
}

impl<AccountId, BlockNumber> PartialOrd for PriceLevel<AccountId, BlockNumber> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<AccountId, BlockNumber> PartialEq for PriceLevel<AccountId, BlockNumber> {
    fn eq(&self, other: &Self) -> bool {
        self.price_level == other.price_level
    }
}

impl<AccountId, BlockNumber> Eq for PriceLevel<AccountId, BlockNumber> {}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct OrderBook<AccountId, BlockNumber, AssetId> {
    // Unique ID that identifies a orderbook in storage
    pub(crate) id: TradingPair,
    // notation BTC/ETH
    // BTC -- AssetId from GenericAsset
    pub(crate) trading_asset: AssetId,  // pallet_generic_asset AssetID for trading asset
    // ETH -- AssetId from GenericAsset
    pub(crate) base_asset: AssetId, // pallet_generic_asset AssetID for trading asset
    pub(crate) nonce: u64, // Relic from previous version. Not used.
    // Contains all the active orders, orders are removed once they are cancelled or fulfilled completely
    pub(crate) orders: btree_map::BTreeMap<Vec<u8>, Order<AccountId, BlockNumber>>,
    // Advanced orders are not needed now. Just leave it here.
    // It is going to be used in the future.
    pub(crate) advanced_bid_orders: BinaryHeap<PriceLevel<AccountId, BlockNumber>>,
    pub(crate) advanced_ask_orders: BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator>,
    // maximum BinaryHeap of Pricelevel Struct for Bid orders
    pub(crate) bids: BinaryHeap<PriceLevel<AccountId, BlockNumber>>,
    // minimum BinaryHeap of Pricelevel Struct for asks orders
    pub(crate) asks: BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator>,
    // To store closing_ask_price, closing_bid_price and volume of trades executed between blocks
    pub(crate) market_data: Vec<MarketData<BlockNumber>>,
    pub(crate) enabled: bool,  // Relic from previous version. Not need per say.
}

impl<AccountId, BlockNumber, AssetId> OrderBook<AccountId, BlockNumber, AssetId> {
    pub fn get_trading_asset(&self) -> &AssetId {
        return &self.trading_asset;
    }

    pub fn get_base_asset(&self) -> &AssetId {
        return &self.base_asset;
    }

    pub fn get_orders(&self) -> &btree_map::BTreeMap<Vec<u8>, Order<AccountId, BlockNumber>> {
        return &self.orders;
    }

    pub fn get_asks_mut(&mut self) -> &mut BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {
        return &mut self.asks;
    }

    pub fn get_asks(self) -> BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator>{
        return self.asks
    }

    pub fn get_bids_mut(&mut self) -> &mut BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
        return &mut self.bids;
    }

    pub fn get_bids(self) -> BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
        return self.bids;
    }

    // pub fn set_bids(&mut self, bids: &mut BinaryHeap<PriceLevel<AccountId, BlockNumber>, MaxComparator>) {
    //     &mut self.bids = bids;
    // }

    pub fn get_advanced_asks(self) -> BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {
        return self.advanced_ask_orders;
    }

    pub fn get_advanced_bids(self) -> BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
        return self.advanced_bid_orders;
    }
}

#[derive(Encode, Decode, Clone, Debug, Default)]
pub struct MarketData<BlockNumber> {
    pub(crate) current_block: BlockNumber, // Data Included is for this block number
    pub(crate) closing_bid: FixedU128,
    pub(crate) closing_ask: FixedU128,
    pub(crate) volume: FixedU128, // Total volume of trades executed in this block ( total quantity traded in trading_asset)
}


// FIXME(The given implementation is not correct and needs to be fixed later)
impl<BlockNumber> Ord for MarketData<BlockNumber> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<BlockNumber> PartialOrd for MarketData<BlockNumber> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<BlockNumber> PartialEq for MarketData<BlockNumber> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl<BlockNumber> Eq for MarketData<BlockNumber> {}


impl<AccountId, BlockNumber> Ord for BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<AccountId, BlockNumber> PartialOrd for BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl<AccountId, BlockNumber> PartialEq for BinaryHeap<PriceLevel<AccountId, BlockNumber>> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl<AccountId, BlockNumber> Eq for BinaryHeap<PriceLevel<AccountId, BlockNumber>> {}

// For MinComparator Binary-Heap
impl<AccountId, BlockNumber> Ord for BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<AccountId, BlockNumber> PartialOrd for BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl<AccountId, BlockNumber> PartialEq for BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl<AccountId, BlockNumber> Eq for BinaryHeap<PriceLevel<AccountId, BlockNumber>, MinComparator> {}



