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

cargo fmt --check || exit
RUSTFLAGS="-D warnings" cargo build || exit
cargo build --features try-runtime || exit
cargo build --features runtime-benchmarks || exit
./target/debug/polkadex-node benchmark pallet --pallet "*" --extrinsic "*" --steps 2 --repeat 1 || exit
cargo clippy -- -D warnings || exit
cargo test || exit
RUSTFLAGS="-D warnings" cargo build -p thea-message-handler || exit
