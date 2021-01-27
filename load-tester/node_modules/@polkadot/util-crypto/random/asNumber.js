"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.randomAsNumber = randomAsNumber;

var _bn = _interopRequireDefault(require("bn.js"));

var _util = require("@polkadot/util");

var _asHex = require("./asHex");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const BN_53 = new _bn.default(0b11111111111111111111111111111111111111111111111111111);
/**
 * @name randomAsNumber
 * @summary Creates a random number from random bytes.
 * @description
 * Returns a random number generated from the secure bytes.
 * @example
 * <BR>
 *
 * ```javascript
 * import { randomAsNumber } from '@polkadot/util-crypto';
 *
 * randomAsNumber(); // => <random number>
 * ```
 */

function randomAsNumber() {
  return (0, _util.hexToBn)((0, _asHex.randomAsHex)(8)).and(BN_53).toNumber();
}