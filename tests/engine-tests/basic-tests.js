// Import
const {ApiPromise, WsProvider, Keyring} = require('@polkadot/api');
// Crypto promise, package used by keyring internally
const {cryptoWaitReady} = require('@polkadot/util-crypto');


// Construct
const wsProvider = new WsProvider('ws://0.0.0.0:9944');
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
    const tradingPairID = "0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9";
    const UNIT = 1000000000000;
    let options = {
        permissions: {
            update: null,
            mint: null,
            burn: null
        }
    }
    // Create first token - Say USDT
    await api.tx.genericAsset.create([1000 * UNIT, options]).signAndSend(alice, {nonce: 0});
    // Create second token - Say BTC
    await api.tx.genericAsset.create([UNIT, options]).signAndSend(bob, {nonce: 0});
    // Note token created first has Token ID as 1 and second token has ID 2.
    // Create the tradingPair BTC/USDT - (2,1)
    await api.tx.polkadex.registerNewOrderbook(2, 1).signAndSend(alice, { nonce: 1 });

    // Let's place some orders and check if we got the expected results
    // Alice places buy limit orders
    await api.tx.polkadex.submitOrder("BidLimit", tradingPairID, 820*UNIT, 0.2*UNIT).signAndSend(alice,{ nonce: 2 });
    await api.tx.polkadex.submitOrder("BidLimit", tradingPairID, 800*UNIT, 0.1*UNIT).signAndSend(alice,{ nonce: 3 });
    await api.tx.polkadex.submitOrder("BidLimit", tradingPairID, 850*UNIT, 0.1*UNIT).signAndSend(alice,{ nonce: 4 });
    await api.tx.polkadex.submitOrder("BidLimit", tradingPairID, 840*UNIT, 0.1*UNIT).signAndSend(alice,{ nonce: 5 });
    await api.tx.polkadex.submitOrder("BidLimit", tradingPairID, 900*UNIT, 0.1*UNIT).signAndSend(alice,{ nonce: 6 });
    // Bob places sell limit orders
    await api.tx.polkadex.submitOrder("AskLimit", tradingPairID, 1075*UNIT, 0.2*UNIT).signAndSend(bob,{ nonce: 1 });
    await api.tx.polkadex.submitOrder("AskLimit", tradingPairID, 1100*UNIT, 0.1*UNIT).signAndSend(bob,{ nonce: 2 });
    await api.tx.polkadex.submitOrder("AskLimit", tradingPairID, 1060*UNIT, 0.1*UNIT).signAndSend(bob,{ nonce: 3 });
    await api.tx.polkadex.submitOrder("AskLimit", tradingPairID, 1040*UNIT, 0.1*UNIT).signAndSend(bob,{ nonce: 4 });
    await api.tx.polkadex.submitOrder("AskLimit", tradingPairID, 1000*UNIT, 0.1*UNIT).signAndSend(bob,{ nonce: 5 });

    // Query the storage about the values
    // Test #1
    await api.query.genericAsset.freeBalance(1,alice.address, (balance)=>{
        console.log("Free balance of Alice: ",balance.toNumber());
        if (balance.toNumber() === 497*UNIT){
           console.log("ALICE-FREE-BALANCE-TEST: Passed")
        }else{
            console.log("ALICE-FREE-BALANCE-TEST: Failed")
        }
    })
    // Test #2
    await api.query.genericAsset.freeBalance(2,bob.address, (balance)=>{
        console.log("Free balance of Alice: ",balance.toNumber());
        if (balance.toNumber() === 0.4*UNIT){
            console.log("BOB-FREE-BALANCE-TEST: Passed")
        }else{
            console.log("BOB-FREE-BALANCE-TEST: Failed")
        }
    })
    // Test #3
    await api.query.genericAsset.reservedBalance(1,alice.address, (balance)=>{
        console.log("Reserved balance of Alice: ",balance.toNumber());
        if (balance.toNumber() === 503*UNIT){
            console.log("ALICE-RESERVED-BALANCE-TEST: Passed")
        }else{
            console.log("ALICE-RESERVED-BALANCE-TEST: Failed")
        }
    })
    // Test #4
    await api.query.genericAsset.reservedBalance(2,bob.address, (balance)=>{
        console.log("Reserved balance of Alice: ",balance.toNumber());
        if (balance.toNumber() === 0.6*UNIT){
            console.log("BOB-RESERVED-BALANCE-TEST: Passed")
        }else{
            console.log("BOB-RESERVED-BALANCE-TEST: Failed")
        }
    })

    // TODO Add Market Orders

}