../target/release/polkadex-node --validator --thea-dummy-mode --port 30334 --base-path ./validator02 \
    -lthea=trace --ws-port=9945 --rpc-port=9946 --chain=../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30335/p2p/12D3KooWCMKvu1tJKQBjDZ4hN1saTP6D58e4WkwLZwks5cPpxqY7 \
    --node-key=d353c4b01db05aa66ddeab9d85c2fa2252368dd4961606e5985ed1e8f40dbc50