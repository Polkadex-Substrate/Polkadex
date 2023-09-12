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

FROM ubuntu:22.04

RUN apt-get update && apt-get install --assume-yes ca-certificates

COPY ./polkadex-node /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkadex-polkadex-mainnet-polkadex-parachain-node polkadex-polkadex-mainnet-polkadex-parachain-node && \
    mkdir -p /polkadex-polkadex-mainnet-polkadex-parachain-node/.local/share && \
    mkdir /data && \
    chown -R polkadex-polkadex-mainnet-polkadex-parachain-node:polkadex-polkadex-mainnet-polkadex-parachain-node /data && \
    ln -s /data /polkadex-polkadex-mainnet-polkadex-parachain-node/.local/share/polkadex-polkadex-mainnet-polkadex-parachain-node && \
    rm -rf /usr/bin /usr/sbin

COPY ./extras/customSpecRaw.json /data

USER polkadex-node

EXPOSE 30333 9933 9944
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/polkadex-node"]
