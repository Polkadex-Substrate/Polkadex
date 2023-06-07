#!/bin/bash
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

# call this script from repo root directory

# Fail fast if any commands exists with error
# Print all executed commands
set -ex

TOOLCHAIN=$(cat ./rust-toolchain)

# Download rustup script and execute it
curl https://sh.rustup.rs -sSf > ./rustup.sh
chmod +x ./rustup.sh
./rustup.sh -y

# Load new environment
source $HOME/.cargo/env

# Install nightly that supports clippy
# Overview: https://rust-lang.github.io/rustup-components-history/index.html
rustup toolchain add $TOOLCHAIN

# Install aux components, clippy for linter, rustfmt for formatting
rustup component add clippy --toolchain $TOOLCHAIN
rustup component add rustfmt --toolchain $TOOLCHAIN

# Install WASM toolchain
rustup target add wasm32-unknown-unknown --toolchain $TOOLCHAIN

# Install wasm-gc
if ! [ -x "$(command -v wasm-gc)" ]; then
    cargo install --git https://github.com/alexcrichton/wasm-gc
else
    echo "wasm-gc already installed"
fi

# Show the installed versions
rustup show
