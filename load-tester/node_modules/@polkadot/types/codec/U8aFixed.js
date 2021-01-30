"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.U8aFixed = void 0;

var _util = require("@polkadot/util");

var _Raw = require("./Raw");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decodeU8aFixed(value, bitLength) {
  if (Array.isArray(value) || (0, _util.isString)(value)) {
    return decodeU8aFixed((0, _util.u8aToU8a)(value), bitLength);
  } // ensure that we have an actual u8a with the full length as specified by
  // the bitLength input (padded with zeros as required)


  const byteLength = bitLength / 8;
  const sub = value.subarray(0, byteLength);

  if (sub.length === byteLength) {
    return sub;
  }

  const u8a = new Uint8Array(byteLength);
  u8a.set(sub, 0);
  return u8a;
}
/**
 * @name U8aFixed
 * @description
 * A U8a that manages a a sequence of bytes up to the specified bitLength. Not meant
 * to be used directly, rather is should be subclassed with the specific lengths.
 */


class U8aFixed extends _Raw.Raw {
  constructor(registry, value = new Uint8Array(), bitLength = 256) {
    super(registry, decodeU8aFixed(value, bitLength));
  }

  static with(bitLength, typeName) {
    return class extends U8aFixed {
      constructor(registry, value) {
        super(registry, value, bitLength);
      }

      toRawType() {
        return typeName || super.toRawType();
      }

    };
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return `[u8;${this.length}]`;
  }

}

exports.U8aFixed = U8aFixed;