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

echo "Setting Bootnode Keys"
curl http://localhost:10000 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe1"
curl http://localhost:10000 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran1"
curl http://localhost:10000 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob1"
curl http://localhost:10000 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/thea1"
echo "Setting Validator01 Keys"
curl http://localhost:10001 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe2"
curl http://localhost:10001 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran2"
curl http://localhost:10001 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob2"
curl http://localhost:10001 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/thea2"
echo "Setting Validator02 Keys"
curl http://localhost:10002 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe3"
curl http://localhost:10002 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran3"
curl http://localhost:10002 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob3"
curl http://localhost:10002 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/thea3"
