"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Enum = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

var _Null = require("../primitive/Null");

var _Struct = require("./Struct");

var _utils = require("./utils");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
function extractDef(registry, _def) {
  if (!Array.isArray(_def)) {
    const def = (0, _utils.mapToTypeMap)(registry, _def);
    const isBasic = !Object.values(def).some(type => type !== _Null.Null);
    return {
      def,
      isBasic
    };
  }

  return {
    def: _def.reduce((def, key) => {
      def[key] = _Null.Null;
      return def;
    }, {}),
    isBasic: true
  };
}

function createFromValue(registry, def, index = 0, value) {
  const Clazz = Object.values(def)[index];
  (0, _util.assert)(!(0, _util.isUndefined)(Clazz), `Unable to create Enum via index ${index}, in ${Object.keys(def).join(', ')}`);
  return {
    index,
    value: value instanceof Clazz ? value : new Clazz(registry, value)
  };
}

function decodeFromJSON(registry, def, key, value) {
  // JSON comes in the form of { "<type (lowercased)>": "<value for type>" }, here we
  // additionally force to lower to ensure forward compat
  const keys = Object.keys(def).map(k => k.toLowerCase());
  const keyLower = key.toLowerCase();
  const index = keys.indexOf(keyLower);
  (0, _util.assert)(index !== -1, `Cannot map Enum JSON, unable to find '${key}' in ${keys.join(', ')}`);

  try {
    return createFromValue(registry, def, index, value);
  } catch (error) {
    throw new Error(`Enum(${key}):: ${error.message}`);
  }
}

function decodeFromString(registry, def, value) {
  return (0, _util.isHex)(value) // eslint-disable-next-line @typescript-eslint/no-use-before-define
  ? decodeFromValue(registry, def, (0, _util.hexToU8a)(value)) : decodeFromJSON(registry, def, value);
}

function decodeFromValue(registry, def, value) {
  if ((0, _util.isU8a)(value)) {
    return createFromValue(registry, def, value[0], value.subarray(1));
  } else if ((0, _util.isNumber)(value)) {
    return createFromValue(registry, def, value);
  } else if ((0, _util.isString)(value)) {
    return decodeFromString(registry, def, value.toString());
  } else if ((0, _util.isObject)(value)) {
    const key = Object.keys(value)[0];
    return decodeFromJSON(registry, def, key, value[key]);
  } // Worst-case scenario, return the first with default


  return createFromValue(registry, def, 0);
}

function decodeEnum(registry, def, value, index) {
  // NOTE We check the index path first, before looking at values - this allows treating
  // the optional indexes before anything else, more-specific > less-specific
  if ((0, _util.isNumber)(index)) {
    return createFromValue(registry, def, index, value); // eslint-disable-next-line @typescript-eslint/no-use-before-define
  } else if (value instanceof Enum) {
    return createFromValue(registry, def, value.index, value.value);
  } // Or else, we just look at `value`


  return decodeFromValue(registry, def, value);
}
/**
 * @name Enum
 * @description
 * This implements an enum, that based on the value wraps a different type. It is effectively
 * an extension to enum where the value type is determined by the actual index.
 */
// TODO:
//   - As per Enum, actually use TS enum
//   - It should rather probably extend Enum instead of copying code


var _def2 = (0, _classPrivateFieldLooseKey2.default)("def");

var _index = (0, _classPrivateFieldLooseKey2.default)("index");

var _indexes = (0, _classPrivateFieldLooseKey2.default)("indexes");

var _isBasic = (0, _classPrivateFieldLooseKey2.default)("isBasic");

var _raw = (0, _classPrivateFieldLooseKey2.default)("raw");

class Enum {
  constructor(registry, def, value, index) {
    this.registry = void 0;
    Object.defineProperty(this, _def2, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _index, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _indexes, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _isBasic, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _raw, {
      writable: true,
      value: void 0
    });
    const defInfo = extractDef(registry, def);
    const decoded = decodeEnum(registry, defInfo.def, value, index);
    this.registry = registry;
    (0, _classPrivateFieldLooseBase2.default)(this, _def2)[_def2] = defInfo.def;
    (0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic] = defInfo.isBasic;
    (0, _classPrivateFieldLooseBase2.default)(this, _indexes)[_indexes] = Object.keys(defInfo.def).map((_, index) => index);
    (0, _classPrivateFieldLooseBase2.default)(this, _index)[_index] = (0, _classPrivateFieldLooseBase2.default)(this, _indexes)[_indexes].indexOf(decoded.index) || 0;
    (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw] = decoded.value;
  }

  static with(Types) {
    return class extends Enum {
      constructor(registry, value, index) {
        super(registry, Types, value, index);
        Object.keys((0, _classPrivateFieldLooseBase2.default)(this, _def2)[_def2]).forEach(_key => {
          const name = (0, _util.stringUpperFirst)((0, _util.stringCamelCase)(_key.replace(' ', '_')));
          const askey = `as${name}`;
          const iskey = `is${name}`;
          (0, _util.isUndefined)(this[iskey]) && Object.defineProperty(this, iskey, {
            enumerable: true,
            get: () => this.type === _key
          });
          (0, _util.isUndefined)(this[askey]) && Object.defineProperty(this, askey, {
            enumerable: true,
            get: () => {
              (0, _util.assert)(this[iskey], `Cannot convert '${this.type}' via ${askey}`);
              return this.value;
            }
          });
        });
      }

    };
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    return 1 + (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].encodedLength;
  }
  /**
   * @description returns a hash of the contents
   */


  get hash() {
    return this.registry.hash(this.toU8a());
  }
  /**
   * @description The index of the metadata value
   */


  get index() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _index)[_index];
  }
  /**
   * @description true if this is a basic enum (no values)
   */


  get isBasic() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic];
  }
  /**
   * @description Checks if the value is an empty value
   */


  get isEmpty() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].isEmpty;
  }
  /**
   * @description Checks if the Enum points to a [[Null]] type
   */


  get isNone() {
    return this.isNull;
  }
  /**
   * @description Checks if the Enum points to a [[Null]] type (deprecated, use isNone)
   */


  get isNull() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw] instanceof _Null.Null;
  }
  /**
   * @description The available keys for this enum
   */


  get defEntries() {
    return Object.keys((0, _classPrivateFieldLooseBase2.default)(this, _def2)[_def2]);
  }
  /**
   * @description The available keys for this enum
   */


  get defKeys() {
    return Object.keys((0, _classPrivateFieldLooseBase2.default)(this, _def2)[_def2]);
  }
  /**
   * @description The name of the type this enum value represents
   */


  get type() {
    return this.defKeys[(0, _classPrivateFieldLooseBase2.default)(this, _index)[_index]];
  }
  /**
   * @description The value of the enum
   */


  get value() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw];
  }
  /**
   * @description Compares the value of the input to see if there is a match
   */


  eq(other) {
    // cater for the case where we only pass the enum index
    if ((0, _util.isNumber)(other)) {
      return this.toNumber() === other;
    } else if ((0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic] && (0, _util.isString)(other)) {
      return this.type === other;
    } else if ((0, _util.isU8a)(other)) {
      return !this.toU8a().some((entry, index) => entry !== other[index]);
    } else if ((0, _util.isHex)(other)) {
      return this.toHex() === other;
    } else if (other instanceof Enum) {
      return this.index === other.index && this.value.eq(other.value);
    } else if ((0, _util.isObject)(other)) {
      return this.value.eq(other[this.type]);
    } // compare the actual wrapper value


    return this.value.eq(other);
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


  toHuman(isExtended) {
    return (0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic] ? this.type : {
      [this.type]: (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toHuman(isExtended)
    };
  }
  /**
   * @description Converts the Object to JSON, typically used for RPC transfers
   */


  toJSON() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic] ? this.type : {
      [this.type]: (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toJSON()
    };
  }
  /**
   * @description Returns the number representation for the value
   */


  toNumber() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _index)[_index];
  }
  /**
   * @description Returns a raw struct representation of the enum types
   */


  _toRawStruct() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _isBasic)[_isBasic] ? this.defKeys : _Struct.Struct.typesToMap(this.registry, (0, _classPrivateFieldLooseBase2.default)(this, _def2)[_def2]);
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return JSON.stringify({
      _enum: this._toRawStruct()
    });
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    return this.isNull ? this.type : JSON.stringify(this.toJSON());
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    return (0, _util.u8aConcat)(new Uint8Array(isBare ? [] : [(0, _classPrivateFieldLooseBase2.default)(this, _indexes)[_indexes][(0, _classPrivateFieldLooseBase2.default)(this, _index)[_index]]]), (0, _classPrivateFieldLooseBase2.default)(this, _raw)[_raw].toU8a(isBare));
  }

}

exports.Enum = Enum;