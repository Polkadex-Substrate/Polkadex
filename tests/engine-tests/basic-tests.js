// Import
const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
// Crypto promise, package used by keyring internally
const {cryptoWaitReady} = require('@polkadot/util-crypto');


// Construct
const wsProvider = new WsProvider('ws://127.0.0.1:9944');
temp().then(r => console.log("Exited"))

async function temp() {
    // Wait for the promise to resolve, async WASM or `cryptoWaitReady().then(() => { ... })`
    await cryptoWaitReady();

    // Create a keyring instance
    const keyring = new Keyring({type: 'sr25519'});
    const alice = keyring.addFromUri('//Alice', {name: 'Alice default'});
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
                "volume": "FixedU128"
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
            "Address": "AccountId"
        },
    });

    const UNIT = 1000000000000;
    let options = {
        permissions: {
            update: null,
            mint: null,
            burn: null
        }
    }
    // Create first token - Say USDT
    await api.tx.genericAsset.create([1000 * UNIT, options]).signAndSend(alice, {nonce: 0}, (result) => {
        console.log(`Alice's Current status is ${result.status}`);
        if (result.status.isInBlock) {
            console.log(`Create-Token #1 Transaction included at blockHash ${result.status.asInBlock}`);
        } else if (result.status.isFinalized) {
            console.log(`Create-Token #1 Transaction finalized at blockHash ${result.status.asFinalized}`);
        }
    });
    // Create second token - Say BTC
    await api.tx.genericAsset.create([1000 * UNIT, options]).signAndSend(bob, {nonce: 0}, (result) => {
        console.log(`Bob's Current status is ${result.status}`);
        if (result.status.isInBlock) {
            console.log(`Create-Token #2 Transaction included at blockHash ${result.status.asInBlock}`);
        } else if (result.status.isFinalized) {
            console.log(`Create-Token #2 Transaction finalized at blockHash ${result.status.asFinalized}`);
        }
    });

    // Note token created first has Token ID as 1 and second token has ID 2.
    // Create the tradingPair BTC/USDT - (2,1)
    await api.tx.templateModule.registerNewOrderbook(2, 1).signAndSend(alice, { nonce: 1 },(result) => {
        if (result.status.isInBlock) {
            console.log(`RegisterTradingPair Transaction included at blockHash ${result.status.asInBlock}`);
            result.events.forEach(({event: {data, method, section}, phase}) => {
                console.log('Events: ', phase.toString(), `: ${section}.${method}`, data.toString());
            });
        } else if (result.status.isFinalized) {
            console.log(`RegisterTradingPair Transaction finalized at blockHash ${result.status.asFinalized}`);
        }
    });

    // Let's place some orders and check if we got the expected results
    // Alice places a buy order at unit price 1 for 1 unit quantity
    // await api.tx.templateModule.submitOrder("BidLimit", tradingPairID, UNIT, UNIT).signAndSend(alice,{ nonce: 2 });

}