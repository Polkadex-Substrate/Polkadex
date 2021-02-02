"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u8aConcat = u8aConcat;

var _toU8a = require("./toU8a");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name u8aConcat
 * @summary Creates a concatenated Uint8Array from the inputs.
 * @description
 * Concatenates the input arrays into a single `UInt8Array`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { { u8aConcat } from '@polkadot/util';
 *
 * u8aConcat(
 *   new Uint8Array([1, 2, 3]),
 *   new Uint8Array([4, 5, 6])
 * ); // [1, 2, 3, 4, 5, 6]
 * ```
 */
function u8aConcat(...list) {
  let length = 0;
  let offset = 0;
  const u8as = new Array(list.length);

  for (let i = 0; i < list.length; i++) {
    u8as[i] = (0, _toU8a.u8aToU8a)(list[i]);
    length += u8as[i].length;
  }

  const result = new Uint8Array(length);

  for (let i = 0; i < u8as.length; i++) {
    result.set(u8as[i], offset);
    offset += u8as[i].length;
  }

  return result;
}