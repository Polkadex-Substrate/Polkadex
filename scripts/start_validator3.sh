# This file is part of Polkadex.
#
# Copyright (c) 2023 Polkadex oü.
# SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

../target/release/polkadex-node --validator --port 30335 --base-path ./validator03  \
    --ws-port=9947 --rpc-port=9948 --chain=../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh \
    --node-key=24f121a84149f784f9fe3f1e2fb04e8873191a510bc4b073a3a815d78a29cf2d
