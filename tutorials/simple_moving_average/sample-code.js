// IMPORTANT NOTE
// This is a simple tutorial that shows how to retrieve market data from Polkadex nodes in real time
// These data can be used to do technical analysis off-chain and place trades accordingly.
// The given example uses trades from ETH/BTC market of Binance Public API to simulate trades. Binance API was not chosen on
// endorse them but only as an example, It should only be treated as a quick and dirty solution to simulate real trades.


// Import
const {ApiPromise, WsProvider} = require('@polkadot/api');


const wsProvider = new WsProvider('ws://0.0.0.0:9944');
polkadex_market_data().then();


async function polkadex_market_data() {

    const api = await ApiPromise.create({
        provider: wsProvider,
        types: {
            "OrderType": {
                "_enum": [
                    "BidLimit",
                    "BidMarket",
                    "AskLimit",
                    "AskMarket"
                ]
            },
            "Order": {
                "id": "Hash",
                "trading_pair": "Hash",
                "trader": "AccountId",
                "price": "FixedU128",
                "quantity": "FixedU128",
                "order_type": "OrderType"
            },
            "MarketData": {
                "low": "FixedU128",
                "high": "FixedU128",
                "volume": "FixedU128",
                "open": "FixedU128",
                "close": "FixedU128"

            },
            "LinkedPriceLevel": {
                "next": "Option<FixedU128>",
                "prev": "Option<FixedU128>",
                "orders": "Vec<Order>"
            },
            "Orderbook": {
                "trading_pair": "Hash",
                "base_asset_id": "u32",
                "quote_asset_id": "u32",
                "best_bid_price": "FixedU128",
                "best_ask_price": "FixedU128"
            },
            "LookupSource": "AccountId",
            "Address": "AccountId",
            "LinkedPriceLevelRpc": {
                "next": "Vec<u8>",
                "prev": "Vec<u8>",
                "orders": "Vec<Order4RPC>"
            },
            "Order4RPC": {
                "id": "[u8;32]",
                "trading_pair": "[u8;32]",
                "trader": "[u8;32]",
                "price": "Vec<u8>",
                "quantity": "Vec<u8>",
                "order_type": "OrderType"
            }
        },
    });


    const tradingPairID = "0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9";
    const UNIT = 1000000000000;
    const total_issuance = 1000 * UNIT;
    const FixedU128_denominator = 1000000000000000000;

    // Load all historic data to populate the graph
    let currentHeader = await api.rpc.chain.getHeader(); // Get the latest header
    let blocks = [];
    for (let i = 0; i < 2400; i++) {
        blocks.push([tradingPairID,currentHeader.number - i])
    }
    await api.query.polkadex.marketInfo.multi(blocks, (callback) => {
        for(let market_data of callback){
            console.log(`
                 Low: ${market_data.low / FixedU128_denominator}
                 High: ${market_data.high / FixedU128_denominator}
                 Volume: ${market_data.volume / FixedU128_denominator}
                 Open: ${market_data.open / FixedU128_denominator}
                 Close: ${market_data.close / FixedU128_denominator}`);
        }
    });

    /// It will give the real time updates of market data on 3 second interval
    // Now there are some trades executing in the system so now let's listen for market data updates from Polkadex
    api.derive.chain.subscribeNewHeads((header) => {
        api.query.polkadex.marketInfo(tradingPairID, header.number).then(market_data => {
            console.log(`
             BlockNumber: ${header.number}  
             Low: ${market_data.low / FixedU128_denominator}
             High: ${market_data.high / FixedU128_denominator}
             Volume: ${market_data.volume / FixedU128_denominator} 
             Open: ${market_data.open / FixedU128_denominator} 
             Close: ${market_data.close / FixedU128_denominator}`);

        });
    });

}