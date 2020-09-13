use serde::{Deserialize, Serialize};
use pallet_dex::apis;


#[derive(Deserialize,Serialize)]
pub struct OuterPriceLevelData {
    pub(crate) price_level: f64,
    pub(crate) amount: f64,
}

#[derive(Deserialize,Serialize)]
pub struct OuterOrderBookApi {
    pub(crate) bids: Vec<OuterPriceLevelData>,
    pub(crate) asks: Vec<OuterPriceLevelData>,
    pub(crate) enabled: bool,
}

pub fn convert_price_level_data_to_outer_price_level_data(x: apis::PriceLevelData) -> OuterPriceLevelData{

    OuterPriceLevelData{ price_level: x.price_level.to_fraction(), amount: x.amount.to_fraction() }
}

pub fn convert_order_book_api_to_outer_order_book_api(old: apis::OrderBookApi) -> OuterOrderBookApi{
    let mut new_asks = Vec::new();
    for ask in old.asks{
        new_asks.push(convert_price_level_data_to_outer_price_level_data(ask))
    }
    let mut new_bids = Vec::new();
    for bid in old.bids{
        new_bids.push(convert_price_level_data_to_outer_price_level_data(bid))
    }
    OuterOrderBookApi{
        bids: new_bids,
        asks: new_asks,
        enabled: old.enabled
    }
}