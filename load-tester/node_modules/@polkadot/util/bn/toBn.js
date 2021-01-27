"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bnToBn = bnToBn;

var _bn = _interopRequireDefault(require("bn.js"));

var _toBn = require("../hex/toBn");

var _bigInt = require("../is/bigInt");

var _hex = require("../is/hex");

var _toBn2 = require("../is/toBn");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function numberToBn(value) {
  return _bn.default.isBN(value) ? value : (0, _toBn2.isToBn)(value) ? value.toBn() : new _bn.default(value);
}
/**
 * @name bnToBn
 * @summary Creates a BN value from a BN, BigInt, string (base 10 or hex) or number input.
 * @description
 * `null` inputs returns a `0x0` result, BN values returns the value, numbers returns a BN representation.
 * @example
 * <BR>
 *
 * ```javascript
 * import BN from 'bn.js';
 * import { bnToBn } from '@polkadot/util';
 *
 * bnToBn(0x1234); // => BN(0x1234)
 * bnToBn(new BN(0x1234)); // => BN(0x1234)
 * ```
 */


function bnToBn(value) {
  if (!value) {
    return new _bn.default(0);
  } else if ((0, _hex.isHex)(value)) {
    return (0, _toBn.hexToBn)(value.toString());
  } else if ((0, _bigInt.isBigInt)(value)) {
    return new _bn.default(value.toString());
  }

  return numberToBn(value);
}