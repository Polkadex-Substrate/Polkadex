#!/bin/bash
#
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

for id in {1..20}
do
  port=$((9943 + $id))
  echo "Setting $id Keys with RPC: $port"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe$id"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran$id"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob$id"
done
