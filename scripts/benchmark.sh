#!/bin/bash -e
#
# This file is part of Polkadex.
#
# Copyright (c) 2021-2023 Polkadex oü.
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

cargo build --release --features runtime-benchmarks
./target/release/polkadex-node benchmark --chain dev --list


install -d benchout
for i in `./target/release/polkadex-node benchmark --chain dev --list | sed s/,.*// |sort |uniq` ; do
   echo Try $i
   echo ./target/release/polkadex-node benchmark \
      --chain dev \
      --execution wasm \
      --wasm-execution compiled \
      --pallet=$i \
      --extrinsic="*" \
      --steps 50 \
      --repeat 20 \
      --output=benchout/$i.rs
done
#      --template=templates/orml-weight-template.hbs \
