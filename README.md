![Logo](https://github.com/Polkadex-Substrate/Documentation/blob/master/images/Logo.svg)
## What is Polkadex? :rocket:
Polkadex is a Open Source, Decentralized Exchange Platform made using Substrate Blockchain Framework that provides traders with the centralized user experience.
## Why did we do this? :gift:
There are many decentralized exchanges/protocols available in the market for traders but they still prefer to use centralized solutions for their convenience and ease of use knowing very well that their funds are at risk. This is because decentralized solutions are still not user friendly to an average trader. Some of them also have no proper decentralization and also got hacked in the process. We cannot call an exchange decentralized if it can lose or freeze customer funds.

The problems faced by decentralized exchanges are:

* Inadequate UI/UX experience.
* Low liquidity
* Lack of advanced trading features, high-frequency trading, and bots.
* Lack of proper decentralization and interoperability.

To solve the above problems, our goal is to build a fully decentralized, peer-peer, cryptocurrency exchange for the Defi ecosystem in Substrate. The project envisages the creation of a fully decentralized platform for exchanging tokens in a peer-peer, trustless environment, that enables high-frequency trading, high-liquidity, and lightning-fast transaction speed for supporting Defi applications.

In order to address the first problem, we needed to enable features that attract users into the exchange which includes a fast, responsive UI and trading features. It is mainly to attract day traders and retail investors who prefer centralized exchanges due to convenience and speed of execution. The block time of 3s given by the Babe/Grandpa consensus algorithm allows transaction speeds of up to 400/s under test conditions which is more than sufficient to compete with any centralized solutions in the market today. Please check our analysis [here](https://github.com/Gauthamastro/Exchange_Analytics.git).  Since Substrate allows the modular implementation of the consensus algorithm, we think a platform like a Substrate will support the future growth of the exchange by changing consensus to accommodate more transactions per second as better ones emerge.

Secondly, the lack of liquidity is addressed by enabling,

1. High-frequency trading using feeless transactions.
2. APIs that enable trading/AMM bots to observe market changes and submit trades.
3. Advanced trading features like stop limit, market limit, Stop loss, Fill/Kill, Post only, TWAP, etc.

Thirdly, proper decentralization and Interoperability are achieved by having a parachain in Polkadot that brings in liquidity from other blockchains and also using ChainBridge protocol that connects directly to the Ethereum network. Hence, traders have two different mechanisms to bring in liquidity.

The value we provide to the Substrate community is,

1. They can build custom UI/UX to connect to our network and create their own custom exchange experience.
2. Traders can contribute their own custom trading algorithms by making use of market data provided by our full nodes.
3. They get a decentralized trading platform to trade Polkadot & Ethereum tokens.
4. This will be one of the first Decentralized exchanges to have High-Frequency Trading bot support using APIs directly from full nodes.
   ![Web3 Grants](https://github.com/Polkadex-Substrate/Documentation/blob/master/images/web3%20foundation_grants_badge_black.svg)
## Build the Polkadex Node 💃

To build Polkadex, you will need a proper Substrate development environment. If you need a refresher setting up your Substrate environment, see [Substrate's Getting Started Guide](https://substrate.dev/docs/en/knowledgebase/getting-started/).

Note that cloning master might result in an unstable build. If you want a stable version, check out the [latest releases](https://github.com/Polkadex-Substrate/Polkadex/releases).

```bash
# Fetch the code
git clone https://github.com/Polkadex-Substrate/Polkadex.git
cd Polkadex

# Build the node (The first build will be long (~30min))
cargo build --release
```

If a cargo not found error shows up in the terminal, manually add Rust to your system path (or restart your system):

```bash
source $HOME/.cargo/env
```

Then, you will want to run the node in dev mode using the following command:

```bash
./target/release/polkadex-node --dev
```

> For people not familiar with Substrate, the --dev flag is a way to run a Substrate-based node in a single node developer configuration for testing purposes. You can learn more about `--dev` in [this Substrate tutorial](https://substrate.dev/docs/en/tutorials/create-your-first-substrate-chain/interact).

When running a node via the binary file, data is stored in a local directory typically located in ~/.local/shared/polkadex-node/chains/development/db. If you want to start a fresh instance of the node, you can either delete the content of the folder, or run the following command inside the polkadex folder:

```bash
./target/release/node-polkadex purge-chain --dev
```

This will remove the data folder, note that all chain data is now lost.

## Connecting Polkadot JS Apps to a Local Polkadex Node
The development node is a Substrate-based node, so you can interact with it using standard Substrate tools. The two provided RPC endpoints are:
- HTTP: `http://127.0.0.1:9933`
- WS: `ws://127.0.0.1:9944`

Start by connecting to it with Polkadot JS Apps. Open a browser to: https://polkadot.js.org/apps/#/explorer. This will open Polkadot JS Apps, which automatically connects to Polkadot MainNet.

Click on the top left corner to open the menu to configure the networks, and then navigate down to open the Development sub-menu. In there, you will want to toggle the "Local Node" option, which points Polkadot JS Apps to ws://127.0.0.1:9944. Next, select the Switch button, and the site should connect to your Polkadex development node.

## Documentation :books:
For Tutorials, Documentation and API Reference please check this [page](https://github.com/Polkadex-Substrate/Documentation)
## Contribute :heart_eyes:
We would love to work with anyone who can contribute their work and improve this project. The details will be shared soon.
## License :scroll:
Licensed Under [GPLv3](https://github.com/Polkadex-Substrate/Polkadex/blob/master/LICENSE)