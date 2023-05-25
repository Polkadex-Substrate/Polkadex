../target/release/polkadex-node --validator --thea-dummy-mode --port 30335 --base-path ./validator03  \
    --ws-port=9947 --rpc-port=9948 --chain=../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh \
    --node-key=24f121a84149f784f9fe3f1e2fb04e8873191a510bc4b073a3a815d78a29cf2d