"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bnMin = bnMin;

var _util = require("./util");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name bnMin
 * @summary Finds and returns the smallest value in an array of BNs.
 * @example
 * <BR>
 *
 * ```javascript
 * import BN from 'bn.js';
 * import { bnMin } from '@polkadot/util';
 *
 * bnMin([new BN(1), new BN(3), new BN(2)]).toString(); // => '1'
 * ```
 */
function bnMin(...items) {
  return (0, _util.checkMaxMin)('min', items);
}