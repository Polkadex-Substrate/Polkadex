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

// Initialize Binance
const Binance = require('node-binance-api');
const binance = new Binance().options({
    APIKEY: '<key>',
    APISECRET: '<secret>'
});


const wsProvider = new WsProvider('ws://0.0.0.0:9944');
polkadex_market_data().then();


async function polkadex_market_data() {
    // Wait for the promise to resolve, async WASM or `cryptoWaitReady().then(() => { ... })`
    await cryptoWaitReady();

    // Create a keyring instance
    const keyring = new Keyring({type: 'sr25519'});
    // The create new instance of Alice
    const alice = keyring.addFromUri('//Alice', {name: 'Alice default'});
    // Let's call in Bob too.
    const bob = keyring.addFromUri('//Bob', {name: 'Bob default'});

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
    const total_issuance = 1000*UNIT;
    let options = {
        permissions: {
            update: null,
            mint: null,
            burn: null
        }
    }
    // Create first token - Say USDT
    await api.tx.genericAsset.create([total_issuance, options]).signAndSend(alice, {nonce: 0});
    // Create second token - Say BTC
    await api.tx.genericAsset.create([total_issuance, options]).signAndSend(bob, {nonce: 0});
    // Note token created first has Token ID as 1 and second token has ID 2.
    // Create the tradingPair BTC/USDT - (2,1)
    await api.tx.polkadex.registerNewOrderbook(2, 1).signAndSend(alice, {nonce: 1});


    // Let's simulate some traders
    let alice_nonce = 2;
    let bob_nonce = 1;
    binance.websockets.trades(['ETHBTC'], (trades) => {
        let {e:eventType, E:eventTime, s:symbol, p:price, q:quantity, m:maker, a:tradeId} = trades;
        // console.info(symbol+" trade update. price: "+price+", quantity: "+quantity+", BUY: "+maker);
        if(maker === true){
            api.tx.polkadex.submitOrder("BidLimit", tradingPairID, (parseFloat(price) * UNIT),
                    (parseFloat(quantity) * UNIT)).signAndSend(alice, {nonce: alice_nonce});
            alice_nonce = alice_nonce +1;
        }else{
            api.tx.polkadex.submitOrder("AskLimit", tradingPairID, (parseFloat(price) * UNIT),
                    (parseFloat(quantity) * UNIT)).signAndSend(bob, {nonce: bob_nonce});
            bob_nonce = bob_nonce +1;
        }
    });

}
