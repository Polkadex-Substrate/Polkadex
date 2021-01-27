"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.GenericMultiAddress = void 0;

var _util = require("@polkadot/util");

var _utilCrypto = require("@polkadot/util-crypto");

var _Enum = require("../codec/Enum");

var _AccountId = require("./AccountId");

var _AccountIndex = require("./AccountIndex");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
function decodeMultiU8a(registry, value) {
  if ((0, _util.isU8a)(value) && value.length <= 32) {
    if (value.length === 32) {
      return {
        id: value
      };
    } else if (value.length === 20) {
      return {
        Address20: value
      };
    } else {
      return decodeMultiAny(registry, registry.createType('AccountIndex', value));
    }
  }

  return value;
}

function decodeMultiAny(registry, value) {
  if (value instanceof GenericMultiAddress) {
    return value;
  } else if (value instanceof _AccountId.GenericAccountId) {
    return {
      Id: value
    };
  } else if (value instanceof _AccountIndex.GenericAccountIndex || (0, _util.isNumber)(value) || (0, _util.isBn)(value)) {
    return {
      Index: registry.createType('Compact<AccountIndex>', value)
    };
  } else if ((0, _util.isString)(value)) {
    return decodeMultiU8a(registry, (0, _utilCrypto.decodeAddress)(value.toString()));
  }

  return decodeMultiU8a(registry, value);
}

class GenericMultiAddress extends _Enum.Enum {
  constructor(registry, value) {
    super(registry, {
      Id: 'AccountId',
      Index: 'Compact<AccountIndex>',
      Raw: 'Bytes',
      // eslint-disable-next-line sort-keys
      Address32: 'H256',
      // eslint-disable-next-line sort-keys
      Address20: 'H160'
    }, decodeMultiAny(registry, value));
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    return this.value.toString();
  }

}

exports.GenericMultiAddress = GenericMultiAddress;