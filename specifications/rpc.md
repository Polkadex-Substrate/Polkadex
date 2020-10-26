# RPC Details
## Query Ask Level in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_ask_level",
      "params": ["<block hash at which we need to get the ask_levels>","< trading pair ID in hex>"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the ask_levels  |
| Trading_pair_ID  | Trading pair ID in hex  |

### Success Response
```bash
Returns a list of all different ask price levels available in the orderbook for the given tradingpair
Vec<FixedU128>
```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI

## Query Bid Level in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_bid_level",
      "params": ["<block hash at which we need to get the bid_levels>","< trading pair ID in hex>"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the bid_levels  |
| Trading_pair_ID  | Trading pair ID in hex  |

### Success Response
```bash
Returns a list of all different bid price levels available in the orderbook for the given tradingpair
Vec<FixedU128>
```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI

## Query Price Level in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_price_level",
      "params": ["<block hash at which we need to get the price_level>","< trading pair ID in hex>"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the price_levels  |
| Trading_pair_ID  | Trading pair ID in hex  |


### Success Response
```bash
Returns the struct LinkedPriceLevelRpc
pub struct LinkedPriceLevelRpc {
    next: Vec<u8>,
    prev: Vec<u8>,
    orders: Vec<Order4RPC>,
}

```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI

## Query Orderbook in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_orderbook",
      "params": ["<block hash at which we need to get the Order_book>","< trading pair ID in hex>"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the order_book  |
| Trading_pair_ID  | Trading pair ID in hex  |

### Success Response
```bash
Returns the struct OrderbookRpc
pub struct OrderbookRpc {
    trading_pair: [u8; 32],
    base_asset_id: u32,
    quote_asset_id: u32,
    best_bid_price: Vec<u8>,
    best_ask_price: Vec<u8>,
}
```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI
## Query all Orderbook in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_all_orderbook",
      "params": ["<block hash at which we need to get the Order_book>"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the order_books  |

### Success Response
```bash
List of Orderbooks  available in the Polkadex
pub struct OrderbookRpc {
   trading_pair: [u8; 32], 
   base_asset_id: u32,
   quote_asset_id: u32,
   best_bid_price: Vec<u8>,  
   best_ask_price: Vec<u8>,
}
```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI
## Query Market-Info in the System
### Request
```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_market_info",
      "params": ["<block hash at which we need to get the market_info>","< trading pair ID in hex>"."< blocknum >"]
    }'
```
### Header
| Header  | 
| ------------- | 
| jsonrpc  | 
| id |
| method | 

### Parameters 
| Parameter  | Description |
| ------------- | ------------- |
| Block_Hash  | block hash at which we need to get the Market-Info  |
| Trading_pair_ID  | Trading pair ID in hex  |
| block_num | Block number at which you want to find Market-Info |

### Success Response
```bash
Returns the struct MarketDataRpc
pub struct MarketDataRpc {
    low: Vec<u8>,
    high: Vec<u8>,
    volume: Vec<u8>,
}
```
### Failure response
* 1000 - IdMustBe32Byte
* 1001 - AssetIdConversionFailed
* 1002 - Fixedu128tou128conversionFailed
* 1003 - NoElementFound
* 1004 - ServerErrorWhileCallingAPI
