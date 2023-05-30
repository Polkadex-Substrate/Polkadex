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

../target/release/polkadex-node --validator --port 30334 --base-path ./validator02 \
    -lthea=trace --ws-port=9945 --rpc-port=9946 --chain=../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30335/p2p/12D3KooWCMKvu1tJKQBjDZ4hN1saTP6D58e4WkwLZwks5cPpxqY7 \
    --node-key=d353c4b01db05aa66ddeab9d85c2fa2252368dd4961606e5985ed1e8f40dbc50
