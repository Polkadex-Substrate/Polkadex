"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Option = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

var _Null = require("../primitive/Null");

var _utils = require("./utils");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decodeOptionU8a(registry, Type, value) {
  return !value.length || value[0] === 0 ? new _Null.Null(registry) : new Type(registry, value.subarray(1));
}
/** @internal */


function decodeOption(registry, typeName, value) {
  if ((0, _util.isNull)(value) || (0, _util.isUndefined)(value) || value instanceof _Null.Null) {
    return new _Null.Null(registry);
  }

  const Type = (0, _utils.typeToConstructor)(registry, typeName); // eslint-disable-next-line @typescript-eslint/no-use-before-define

  if (value instanceof Option) {
    return decodeOption(registry, Type, value.value);
  } else if (value instanceof Type) {
    // don't re-create, use as it (which also caters for derived types)
    return value;
  } else if ((0, _util.isU8a)(value)) {
    // the isU8a check happens last in the if-tree - since the wrapped value
    // may be an instance of it, so Type and Option checks go in first
    return decodeOptionU8a(registry, Type, value);
  }

  return new Type(registry, value);
}
/**
 * @name Option
 * @description
 * An Option is an optional field. Basically the first byte indicates that there is
 * is value to follow. If the byte is `1` there is an actual value. So the Option
 * implements that - decodes, checks for optionality and wraps the required structure
 * with a value if/as required/found.
 */


var _Type = (0, _classPrivateFieldLooseKey2.default)("Type");

var _raw = (0, _classPrivateFieldLooseKey2.default)("raw");

class Option {
  constructor(registry, typeName, value) {
    this.registry = void 0;
    Object.defineProperty(this, _Type, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _raw, {
      writable: true,
      value: void 0
    });
    this.registry = registry;
    (0, _classPrivateFieldLooseBase2.default)(this, _Type)[_Type] = (0, _utils.typeToConstructor)(registry, typeName);
    (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw] = decodeOption(registry, typeName, value);
  }

  static with(Type) {
    return class extends Option {
      constructor(registry, value) {
        super(registry, Type, value);
      }

    };
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    // boolean byte (has value, doesn't have) along with wrapped length
    return 1 + (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].encodedLength;
  }
  /**
   * @description returns a hash of the contents
   */


  get hash() {
    return this.registry.hash(this.toU8a());
  }
  /**
   * @description Checks if the Option has no value
   */


  get isEmpty() {
    return this.isNone;
  }
  /**
   * @description Checks if the Option has no value
   */


  get isNone() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw] instanceof _Null.Null;
  }
  /**
   * @description Checks if the Option has a value
   */


  get isSome() {
    return !this.isNone;
  }
  /**
   * @description The actual value for the Option
   */


  get value() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw];
  }
  /**
   * @description Compares the value of the input to see if there is a match
   */


  eq(other) {
    if (other instanceof Option) {
      return this.isSome === other.isSome && this.value.eq(other.value);
    }

    return this.value.eq(other);
  }
  /**
   * @description Returns a hex string representation of the value
   */


  toHex() {
    // This attempts to align with the JSON encoding - actually in this case
    // the isSome value is correct, however the `isNone` may be problematic
    return this.isNone ? '0x' : (0, _util.u8aToHex)(this.toU8a().subarray(1));
  }
  /**
   * @description Converts the Object to to a human-friendly JSON, with additional fields, expansion and formatting of information
   */


  toHuman(isExtended) {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toHuman(isExtended);
  }
  /**
   * @description Converts the Object to JSON, typically used for RPC transfers
   */


  toJSON() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toJSON();
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType(isBare) {
    const wrapped = this.registry.getClassName((0, _classPrivateFieldLooseBase2.default)(this, _Type)[_Type]) || new ((0, _classPrivateFieldLooseBase2.default)(this, _Type)[_Type])(this.registry).toRawType();
    return isBare ? wrapped : `Option<${wrapped}>`;
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toString();
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    if (isBare) {
      return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toU8a(true);
    }

    const u8a = new Uint8Array(this.encodedLength);

    if (this.isSome) {
      u8a.set([1]);
      u8a.set((0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toU8a(), 1);
    }

    return u8a;
  }
  /**
   * @description Returns the value that the Option represents (if available), throws if null
   */


  unwrap() {
    (0, _util.assert)(this.isSome, 'Option: unwrapping a None value');
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw];
  }
  /**
   * @description Returns the value that the Option represents (if available) or defaultValue if none
   * @param defaultValue The value to return if the option isNone
   */


  unwrapOr(defaultValue) {
    return this.isSome ? this.unwrap() : defaultValue;
  }
  /**
   * @description Returns the value that the Option represents (if available) or defaultValue if none
   * @param defaultValue The value to return if the option isNone
   */


  unwrapOrDefault() {
    return this.isSome ? this.unwrap() : new ((0, _classPrivateFieldLooseBase2.default)(this, _Type)[_Type])(this.registry);
  }

}

exports.Option = Option;