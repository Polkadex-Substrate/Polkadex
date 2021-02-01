// IMPORTANT NOTE
// This is a simple tutorial that shows how to retrieve market data from Polkadex nodes in real time
// These data can be used to do technical analysis off-chain and place trades accordingly.
// The given example uses trades from ETH/BTC market of Binance Public API to simulate trades. Binance API was not chosen on
// endorse them but only as an example, It should only be treated as a quick and dirty solution to simulate real trades.
// Polkadex team is not associated with Binance in any way.
// Import
const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
// Crypto promise, package used by keyring internally
const {cryptoWaitReady} = require('@polkadot/util-crypto');
const BN = require("bn.js")
// Initialize Binance
const Binance = require('node-binance-api');
const binance = new Binance().options({
    APIKEY: '<key>',
    APISECRET: '<secret>'
});
const wsProvider = new WsProvider('ws://localhost:9944');
polkadex_market_data().then();
async function polkadex_market_data() {
    // Wait for the promise to resolve, async WASM or `cryptoWaitReady().then(() => { ... })`
    await cryptoWaitReady();
    // Create a keyring instance
    const keyring = new Keyring({type: 'sr25519'});
    // The create new instance of Alice
    const alice = keyring.addFromUri('//Alice', {name: 'Alice default'});
    const api = await ApiPromise.create({
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
            "Order4RPC":{
                "id": "[u8;32]",
                "trading_pair": "[u8;32]",
                "trader": "[u8;32]",
                "price": "Vec<u8>",
                "quantity": "Vec<u8>",
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
            "LinkedPriceLevelRpc":{
                "next": "Vec<u8>",
                "prev": "Vec<u8>",
                "orders": "Vec<Order4RPC>"
            },
            "Orderbook": {
                "trading_pair": "Hash",
                "base_asset_id": "u32",
                "quote_asset_id": "u32",
                "best_bid_price": "FixedU128",
                "best_ask_price": "FixedU128"
            },
            "OrderbookRPC":{
                "trading_pair": "[u8;32]",
                "base_asset_id": "u32",
                "quote_asset_id": "u32",
                "best_bid_price": "Vec<u8>",
                "best_ask_price": "Vec<u8>"
            },
            "OrderbookUpdates": {
                "bids": "Vec<FrontendPricelevel>",
                "asks": "Vec<FrontendPricelevel>"
            },
            "FrontendPricelevel": {
                "price": "FixedU128",
                "quantity": "FixedU128"
            },
            "Permissions": {
                "_enum": [
                    "SystemLevel",
                    "FoundationLevel",
                    "UserLevel"
                ]
            },
            "AssetInfo": {
                "total_issuance": "FixedU128",
                "issuer":"AccountId",
                "permissions":"Permissions",
                "existential_deposit": "FixedU128"
            },
            "AssetID": "H256",
            "Judgement": {
                "_enum": [
                    "Reasonable",
                    "KnownGood",
                    "OutOfDate",
                    "PolkadexFoundationAccount",
                    "Default",
                    "Freeze",
                ]
            },
        },
        rpc: {
            polkadex: {
                getAllOrderbook: {
                    description: " Blah",
                    params: [],
                    type: "Vec<OrderbookRpc>"
                },
                getAskLevel: {
                    description: " Blah",
                    params: [
                        {
                            name: "trading_pair",
                            type: "Hash"
                        }
                    ],
                    type: "Vec<FixedU128>"
                },
                getBidLevel: {
                    description: " Blah",
                    params: [
                        {
                            name: "trading_pair",
                            type: "Hash"
                        }
                    ],
                    type: "Vec<FixedU128>"
                },
                getMarketInfo: {
                    description: " Blah",
                    params: [
                        {
                            name: "trading_pair",
                            type: "Hash"
                        },
                        {
                            name: "blocknum",
                            type: "u32"
                        }
                    ],
                    type: "MarketDataRpc"
                },
                getOrderbook: {
                    description: " Blah",
                    params: [
                        {
                            name: "trading_pair",
                            type: "Hash"
                        }
                    ],
                    type: "OrderbookRpc"
                },
                getOrderbookUpdates: {
                    description: "Gets best 10 bids & asks",
                    params: [
                        {
                            name: "at",
                            type: "Hash"
                        },
                        {
                            name: "trading_pair",
                            type: "Hash"
                        }
                    ],
                    type: "OrderbookUpdates"
                },
                getPriceLevel: {
                    description: " Blah",
                    params: [
                        {
                            name: "trading_pair",
                            type: "Hash"
                        }
                    ],
                    type: "Vec<LinkedPriceLevelRpc>"
                },
            }
        },
        provider: wsProvider
    });
    const tradingPairID = "0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9";
    const UNIT = new BN(1000000000000,10);
    const total_issuance = 100000000000;
    let options = {
        permissions: {
            update: null,
            mint: null,
            burn: null
        }
    }


    api.query.system.events((events) => {
        console.log(`\nReceived ${events.length} events:`);

        // Loop through the Vec<EventRecord>
        events.forEach((record) => {
          // Extract the phase, event and the event types
          const { event, phase } = record;
          const types = event.typeDef;



          // Show what we are busy with
          console.log(`\t${event.section}:${event.method}:: (phase=${phase.toString()})`);
          console.log(`\t\t${event.meta.documentation.toString()}`);

          // Loop through each of the parameters, displaying the type and data
          event.data.forEach((data, index) => {
            console.log(`\t\t\t${types[index].type}: ${data.toString()}`);
          });
        });
      });

      // Create first token - Say USDT
          await api.tx.customAsset.createToken(total_issuance, 0).signAndSend(alice, {nonce: 0}, (status)=>{
              console.log(status.status.toHuman());

          });
          await api.tx.customAsset.createToken(total_issuance, 0).signAndSend(alice, {nonce: 1}, (status)=>{
                  console.log(status.status.toHuman());
          });
}
