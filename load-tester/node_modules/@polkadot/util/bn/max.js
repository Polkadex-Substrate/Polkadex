"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bnMax = bnMax;

var _util = require("./util");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name bnMax
 * @summary Finds and returns the highest value in an array of BNs.
 * @example
 * <BR>
 *
 * ```javascript
 * import BN from 'bn.js';
 * import { bnMax } from '@polkadot/util';
 *
 * bnMax([new BN(1), new BN(3), new BN(2)]).toString(); // => '3'
 * ```
 */
function bnMax(...items) {
  return (0, _util.checkMaxMin)('max', items);
}