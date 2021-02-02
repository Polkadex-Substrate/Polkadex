"use strict";

var _crypto = require("./crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
// start init process immediately
(0, _crypto.cryptoWaitReady)().catch(() => {// shouldn't happen, logged above
});