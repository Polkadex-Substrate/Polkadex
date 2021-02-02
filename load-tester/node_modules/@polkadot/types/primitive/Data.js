"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Data = void 0;

var _util = require("@polkadot/util");

var _Enum = require("../codec/Enum");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decodeDataU8a(registry, value) {
  const indicator = value[0];

  if (!indicator) {
    return [undefined, undefined];
  } else if (indicator >= 1 && indicator <= 33) {
    const length = indicator - 1;
    const data = value.subarray(1, length + 1); // in this case, we are passing a Raw back (since we have no length)

    return [registry.createType('Raw', data), 1];
  } else if (indicator >= 34 && indicator <= 37) {
    return [value.subarray(1, 32 + 1), indicator - 32]; // 34 becomes 2
  }

  throw new Error(`Unable to decode Data, invalid indicator byte ${indicator}`);
}
/** @internal */


function decodeData(registry, value) {
  if (!value) {
    return [undefined, undefined];
  } else if ((0, _util.isU8a)(value) || (0, _util.isString)(value)) {
    return decodeDataU8a(registry, (0, _util.u8aToU8a)(value));
  } // assume we have an Enum or an  object input, handle this via the normal Enum decoding


  return [value, undefined];
}
/**
 * @name Data
 * @description
 * A [[Data]] container with node, raw or hashed data
 */


class Data extends _Enum.Enum {
  constructor(registry, value) {
    super(registry, {
      None: 'Null',
      // 0
      Raw: 'Bytes',
      // 1
      // eslint-disable-next-line sort-keys
      BlakeTwo256: 'H256',
      // 2
      Sha256: 'H256',
      // 3
      // eslint-disable-next-line sort-keys
      Keccak256: 'H256',
      // 4
      ShaThree256: 'H256' // 5

    }, ...decodeData(registry, value));
  }

  get asRaw() {
    return this.value;
  }

  get asSha256() {
    return this.value;
  }

  get isRaw() {
    return this.index === 1;
  }

  get isSha256() {
    return this.index === 3;
  }
  /**
   * @description The encoded length
   */


  get encodedLength() {
    return this.toU8a().length;
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   */


  toU8a() {
    if (this.index === 0) {
      return new Uint8Array(1);
    } else if (this.index === 1) {
      // don't add the length, just the data
      const data = this.value.toU8a(true);
      const length = Math.min(data.length, 32);
      const u8a = new Uint8Array(length + 1);
      u8a.set([data.length + 1], 0);
      u8a.set(data.subarray(0, length), 1);
      return u8a;
    } // otherwise we simply have a hash


    const u8a = new Uint8Array(33);
    u8a.set([this.index + 32], 0);
    u8a.set(this.value.toU8a(), 1);
    return u8a;
  }

}

exports.Data = Data;