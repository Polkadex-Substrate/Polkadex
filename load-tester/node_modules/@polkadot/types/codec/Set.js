"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.CodecSet = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _bn = _interopRequireDefault(require("bn.js"));

var _util = require("@polkadot/util");

var _utils = require("./utils");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
function encodeSet(setValues, value) {
  return value.reduce((result, value) => {
    return result.or((0, _util.bnToBn)(setValues[value] || 0));
  }, new _bn.default(0));
}
/** @internal */


function decodeSetArray(setValues, value) {
  return value.reduce((result, key) => {
    (0, _util.assert)(!(0, _util.isUndefined)(setValues[key]), `Set: Invalid key '${key}' passed to Set, allowed ${Object.keys(setValues).join(', ')}`);
    result.push(key);
    return result;
  }, []);
}
/** @internal */


function decodeSetNumber(setValues, _value) {
  const bn = (0, _util.bnToBn)(_value);
  const result = Object.keys(setValues).reduce((result, key) => {
    if (bn.and((0, _util.bnToBn)(setValues[key])).eq((0, _util.bnToBn)(setValues[key]))) {
      result.push(key);
    }

    return result;
  }, []);
  const computed = encodeSet(setValues, result);
  (0, _util.assert)(bn.eq(computed), `Set: Mismatch decoding '${bn.toString()}', computed as '${computed.toString()}' with ${result.join(', ')}`);
  return result;
}
/** @internal */


function decodeSet(setValues, value = 0, bitLength) {
  (0, _util.assert)(bitLength % 8 === 0, `Expected valid bitLength, power of 8, found ${bitLength}`);
  const byteLength = bitLength / 8;

  if ((0, _util.isString)(value)) {
    return decodeSet(setValues, (0, _util.u8aToU8a)(value), byteLength);
  } else if ((0, _util.isU8a)(value)) {
    return value.length === 0 ? [] : decodeSetNumber(setValues, (0, _util.u8aToBn)(value.subarray(0, byteLength), {
      isLe: true
    }));
  } else if (value instanceof Set || Array.isArray(value)) {
    const input = Array.isArray(value) ? value : [...value.values()];
    return decodeSetArray(setValues, input);
  }

  return decodeSetNumber(setValues, value);
}
/**
 * @name Set
 * @description
 * An Set is an array of string values, represented an an encoded type by
 * a bitwise representation of the values.
 */
// FIXME This is a prime candidate to extend the JavaScript built-in Set


var _allowed = (0, _classPrivateFieldLooseKey2.default)("allowed");

var _byteLength = (0, _classPrivateFieldLooseKey2.default)("byteLength");

class CodecSet extends Set {
  constructor(registry, setValues, value, bitLength = 8) {
    super(decodeSet(setValues, value, bitLength));
    this.registry = void 0;
    Object.defineProperty(this, _allowed, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _byteLength, {
      writable: true,
      value: void 0
    });

    this.add = key => {
      // ^^^ add = () property done to assign this instance's this, otherwise Set.add creates "some" chaos
      // we have the isUndefined(this._setValues) in here as well, add is used internally
      // in the Set constructor (so it is undefined at this point, and should allow)
      (0, _util.assert)((0, _util.isUndefined)((0, _classPrivateFieldLooseBase2.default)(this, _allowed)[_allowed]) || !(0, _util.isUndefined)((0, _classPrivateFieldLooseBase2.default)(this, _allowed)[_allowed][key]), `Set: Invalid key '${key}' on add`);
      super.add(key);
      return this;
    };

    this.registry = registry;
    (0, _classPrivateFieldLooseBase2.default)(this, _allowed)[_allowed] = setValues;
    (0, _classPrivateFieldLooseBase2.default)(this, _byteLength)[_byteLength] = bitLength / 8;
  }

  static with(values, bitLength) {
    return class extends CodecSet {
      constructor(registry, value) {
        super(registry, values, value, bitLength);
        Object.keys(values).forEach(_key => {
          const name = (0, _util.stringUpperFirst)((0, _util.stringCamelCase)(_key));
          const iskey = `is${name}`;
          (0, _util.isUndefined)(this[iskey]) && Object.defineProperty(this, iskey, {
            enumerable: true,
            get: () => this.strings.includes(_key)
          });
        });
      }

    };
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _byteLength)[_byteLength];
  }
  /**
   * @description returns a hash of the contents
   */


  get hash() {
    return this.registry.hash(this.toU8a());
  }
  /**
   * @description true is the Set contains no values
   */


  get isEmpty() {
    return this.size === 0;
  }
  /**
   * @description The actual set values as a string[]
   */


  get strings() {
    return [...super.values()];
  }
  /**
   * @description The encoded value for the set members
   */


  get valueEncoded() {
    return encodeSet((0, _classPrivateFieldLooseBase2.default)(this, _allowed)[_allowed], this.strings);
  }
  /**
   * @description adds a value to the Set (extended to allow for validity checking)
   */


  /**
   * @description Compares the value of the input to see if there is a match
   */
  eq(other) {
    if (Array.isArray(other)) {
      // we don't actually care about the order, sort the values
      return (0, _utils.compareArray)(this.strings.sort(), other.sort());
    } else if (other instanceof Set) {
      return this.eq([...other.values()]);
    } else if ((0, _util.isNumber)(other) || (0, _util.isBn)(other)) {
      return this.valueEncoded.eq((0, _util.bnToBn)(other));
    }

    return false;
  }
  /**
   * @description Returns a hex string representation of the value
   */


  toHex() {
    return (0, _util.u8aToHex)(this.toU8a());
  }
  /**
   * @description Converts the Object to to a human-friendly JSON, with additional fields, expansion and formatting of information
   */


  toHuman() {
    return this.toJSON();
  }
  /**
   * @description Converts the Object to JSON, typically used for RPC transfers
   */


  toJSON() {
    return this.strings;
  }
  /**
   * @description The encoded value for the set members
   */


  toNumber() {
    return this.valueEncoded.toNumber();
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return JSON.stringify({
      _set: (0, _classPrivateFieldLooseBase2.default)(this, _allowed)[_allowed]
    });
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    return `[${this.strings.join(', ')}]`;
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */
  // eslint-disable-next-line @typescript-eslint/no-unused-vars


  toU8a(isBare) {
    return (0, _util.bnToU8a)(this.valueEncoded, {
      bitLength: (0, _classPrivateFieldLooseBase2.default)(this, _byteLength)[_byteLength] * 8,
      isLe: true
    });
  }

}

exports.CodecSet = CodecSet;