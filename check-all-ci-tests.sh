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

cargo fmt --check --features on-chain-release-build || exit
RUSTFLAGS="-D warnings" cargo build --features on-chain-release-build || exit
RUSTFLAGS="-D warnings" cargo build --features try-runtime  || exit
cargo build --features runtime-benchmarks || exit
./target/debug/polkadex-node benchmark pallet --pallet "*" --extrinsic "*" --steps 2 --repeat 1 || exit
cargo clippy -- -D warnings || exit
RUSTFLAGS="-D warnings" cargo test --workspace || exit


# Note while building wasm binary for runtime upgrade use the metadata-hash feature otherwise, ledger app will be broken
# References
# https://forum.polkadot.network/t/polkadot-generic-ledger-app/4295/26
# https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/enable_metadata_hash/index.html
# https://hackmd.io/@ePxWAFa1TbKm0U5Ym3IqgQ/rJdgmf6b0?utm_source=preview-mode&utm_medium=rec
# 0x0e63268eeeddda8e033ade1cc670411d3215fd81be44bdec564932d599d5aee8
# https://www.notion.so/polkadex/Listing-on-Polkadex-Orderbook-3e49fcf22d52474da86dfa65135615e9#b225838c59fa4820a61365e276fd4684