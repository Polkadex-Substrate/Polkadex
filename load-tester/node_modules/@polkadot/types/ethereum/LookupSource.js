"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.GenericEthereumLookupSource = exports.ACCOUNT_ID_PREFIX = void 0;

var _util = require("@polkadot/util");

var _utilCrypto = require("@polkadot/util-crypto");

var _Base = require("../codec/Base");

var _AccountIndex = require("../generic/AccountIndex");

var _AccountId = require("./AccountId");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
const ACCOUNT_ID_PREFIX = new Uint8Array([0xff]);
/** @internal */

exports.ACCOUNT_ID_PREFIX = ACCOUNT_ID_PREFIX;

function decodeString(registry, value) {
  const decoded = (0, _utilCrypto.decodeAddress)(value);
  return decoded.length === 20 ? registry.createType('EthereumAccountId', decoded) : registry.createType('AccountIndex', (0, _util.u8aToBn)(decoded, true));
}
/** @internal */


function decodeU8a(registry, value) {
  // This allows us to instantiate an address with a raw publicKey. Do this first before
  // we checking the first byte, otherwise we may split an already-existent valid address
  if (value.length === 20) {
    return registry.createType('EthereumAccountId', value);
  } else if (value[0] === 0xff) {
    return registry.createType('EthereumAccountId', value.subarray(1));
  }

  const [offset, length] = _AccountIndex.GenericAccountIndex.readLength(value);

  return registry.createType('AccountIndex', (0, _util.u8aToBn)(value.subarray(offset, offset + length), true));
}
/**
 * @name GenericEthereumLookupSource
 * @description
 * A wrapper around an EthereumAccountId and/or AccountIndex that is encoded with a prefix.
 * Since we are dealing with underlying publicKeys (or shorter encoded addresses),
 * we extend from Base with an AccountId/AccountIndex wrapper. Basically the Address
 * is encoded as `[ <prefix-byte>, ...publicKey/...bytes ]` as per spec
 */


class GenericEthereumLookupSource extends _Base.Base {
  constructor(registry, value = new Uint8Array()) {
    super(registry, GenericEthereumLookupSource._decodeAddress(registry, value));
  }
  /** @internal */


  static _decodeAddress(registry, value) {
    return value instanceof GenericEthereumLookupSource ? value._raw : value instanceof _AccountId.GenericEthereumAccountId || value instanceof _AccountIndex.GenericAccountIndex ? value : (0, _util.isBn)(value) || (0, _util.isNumber)(value) || (0, _util.isBigInt)(value) ? registry.createType('AccountIndex', value) : Array.isArray(value) || (0, _util.isHex)(value) || (0, _util.isU8a)(value) ? decodeU8a(registry, (0, _util.u8aToU8a)(value)) : decodeString(registry, value);
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    const rawLength = this._rawLength;
    return rawLength + ( // for 1 byte AccountIndexes, we are not adding a specific prefix
    rawLength > 1 ? 1 : 0);
  }
  /**
   * @description The length of the raw value, either AccountIndex or AccountId
   */


  get _rawLength() {
    return this._raw instanceof _AccountIndex.GenericAccountIndex ? _AccountIndex.GenericAccountIndex.calcLength(this._raw) : this._raw.encodedLength;
  }
  /**
   * @description Returns a hex string representation of the value
   */


  toHex() {
    return (0, _util.u8aToHex)(this.toU8a());
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return 'Address';
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    const encoded = this._raw.toU8a().subarray(0, this._rawLength);

    return isBare ? encoded : (0, _util.u8aConcat)(this._raw instanceof _AccountIndex.GenericAccountIndex ? _AccountIndex.GenericAccountIndex.writeLength(encoded) : ACCOUNT_ID_PREFIX, encoded);
  }

}

exports.GenericEthereumLookupSource = GenericEthereumLookupSource;