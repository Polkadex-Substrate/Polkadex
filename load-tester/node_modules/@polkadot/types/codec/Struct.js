"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Struct = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

var _utils = require("./utils");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decodeStructFromObject(registry, Types, value, jsonMap) {
  let jsonObj;
  return Object.keys(Types).reduce((raw, key, index) => {
    // The key in the JSON can be snake_case (or other cases), but in our
    // Types, result or any other maps, it's camelCase
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    const jsonKey = jsonMap.get(key) && !value[key] ? jsonMap.get(key) : key;

    try {
      if (Array.isArray(value)) {
        // TS2322: Type 'Codec' is not assignable to type 'T[keyof S]'.
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment,@typescript-eslint/no-unsafe-member-access
        raw[key] = value[index] instanceof Types[key] ? value[index] : new Types[key](registry, value[index]);
      } else if (value instanceof Map) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        const mapped = value.get(jsonKey); // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access

        raw[key] = mapped instanceof Types[key] ? mapped : new Types[key](registry, mapped);
      } else if ((0, _util.isObject)(value)) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
        let assign = value[jsonKey];

        if ((0, _util.isUndefined)(assign)) {
          if ((0, _util.isUndefined)(jsonObj)) {
            jsonObj = Object.entries(value).reduce((all, [key, value]) => {
              // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
              all[(0, _util.stringCamelCase)(key)] = value;
              return all;
            }, {});
          } // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment


          assign = jsonObj[jsonKey];
        } // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment,@typescript-eslint/no-unsafe-member-access


        raw[key] = assign instanceof Types[key] ? assign : new Types[key](registry, assign);
      } else {
        throw new Error(`Cannot decode value ${JSON.stringify(value)}`);
      }
    } catch (error) {
      let type = Types[key].name;

      try {
        type = new Types[key](registry).toRawType();
      } catch (error) {// ignore
      }

      throw new Error(`Struct: failed on ${jsonKey}: ${type}:: ${error.message}`);
    }

    return raw;
  }, {});
}
/**
 * Decode input to pass into constructor.
 *
 * @param Types - Types definition.
 * @param value - Value to decode, one of:
 * - null
 * - undefined
 * - hex
 * - Uint8Array
 * - object with `{ key1: value1, key2: value2 }`, assuming `key1` and `key2`
 * are also keys in `Types`
 * - array with `[value1, value2]` assuming the array has the same length as
 * `Object.keys(Types)`
 * @param jsonMap
 * @internal
 */


function decodeStruct(registry, Types, value, jsonMap) {
  if ((0, _util.isHex)(value)) {
    return decodeStruct(registry, Types, (0, _util.hexToU8a)(value), jsonMap);
  } else if ((0, _util.isU8a)(value)) {
    const values = (0, _utils.decodeU8a)(registry, value, Object.values(Types)); // Transform array of values to {key: value} mapping

    return Object.keys(Types).reduce((raw, key, index) => {
      // TS2322: Type 'Codec' is not assignable to type 'T[keyof S]'.
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      raw[key] = values[index];
      return raw;
    }, {});
  } else if (!value) {
    return {};
  } // We assume from here that value is a JS object (Array, Map, Object)


  return decodeStructFromObject(registry, Types, value, jsonMap);
}
/**
 * @name Struct
 * @description
 * A Struct defines an Object with key-value pairs - where the values are Codec values. It removes
 * a lot of repetition from the actual coding, define a structure type, pass it the key/Codec
 * values in the constructor and it manages the decoding. It is important that the constructor
 * values matches 100% to the order in th Rust code, i.e. don't go crazy and make it alphabetical,
 * it needs to decoded in the specific defined order.
 * @noInheritDoc
 */


var _jsonMap = (0, _classPrivateFieldLooseKey2.default)("jsonMap");

var _Types = (0, _classPrivateFieldLooseKey2.default)("Types");

class Struct extends Map {
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  constructor(registry, Types, value = {}, jsonMap = new Map()) {
    super(Object.entries(decodeStruct(registry, (0, _utils.mapToTypeMap)(registry, Types), value, jsonMap)));
    this.registry = void 0;
    Object.defineProperty(this, _jsonMap, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _Types, {
      writable: true,
      value: void 0
    });
    this.registry = registry;
    (0, _classPrivateFieldLooseBase2.default)(this, _jsonMap)[_jsonMap] = jsonMap;
    (0, _classPrivateFieldLooseBase2.default)(this, _Types)[_Types] = (0, _utils.mapToTypeMap)(registry, Types);
  }

  static with(Types, jsonMap) {
    return class extends Struct {
      constructor(registry, value) {
        super(registry, Types, value, jsonMap);
        Object.keys(Types).forEach(key => {
          (0, _util.isUndefined)(this[key]) && Object.defineProperty(this, key, {
            enumerable: true,
            get: () => this.get(key)
          });
        });
      }

    };
  }

  static typesToMap(registry, Types) {
    return Object.entries(Types).reduce((result, [key, Type]) => {
      result[key] = registry.getClassName(Type) || new Type(registry).toRawType();
      return result;
    }, {});
  }
  /**
   * @description The available keys for this enum
   */


  get defKeys() {
    return Object.keys((0, _classPrivateFieldLooseBase2.default)(this, _Types)[_Types]);
  }
  /**
   * @description Checks if the value is an empty value
   */


  get isEmpty() {
    const items = this.toArray();

    for (let i = 0; i < items.length; i++) {
      if (!items[i].isEmpty) {
        return false;
      }
    }

    return true;
  }
  /**
   * @description Returns the Type description to sthe structure
   */


  get Type() {
    return Object.entries((0, _classPrivateFieldLooseBase2.default)(this, _Types)[_Types]).reduce((result, [key, Type]) => {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      result[key] = new Type(this.registry).toRawType();
      return result;
    }, {});
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    return this.toArray().reduce((length, entry) => {
      length += entry.encodedLength;
      return length;
    }, 0);
  }
  /**
   * @description returns a hash of the contents
   */


  get hash() {
    return this.registry.hash(this.toU8a());
  }
  /**
   * @description Compares the value of the input to see if there is a match
   */


  eq(other) {
    return (0, _utils.compareMap)(this, other);
  }
  /**
   * @description Returns a specific names entry in the structure
   * @param name The name of the entry to retrieve
   */


  get(name) {
    return super.get(name);
  }
  /**
   * @description Returns the values of a member at a specific index (Rather use get(name) for performance)
   */


  getAtIndex(index) {
    return this.toArray()[index];
  }
  /**
   * @description Converts the Object to an standard JavaScript Array
   */


  toArray() {
    return [...this.values()];
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
    return [...this.keys()].reduce((json, key) => {
      const value = this.get(key);
      json[key] = value && value.toHuman(isExtended);
      return json;
    }, {});
  }
  /**
   * @description Converts the Object to JSON, typically used for RPC transfers
   */


  toJSON() {
    return [...this.keys()].reduce((json, key) => {
      const jsonKey = (0, _classPrivateFieldLooseBase2.default)(this, _jsonMap)[_jsonMap].get(key) || key;
      const value = this.get(key);
      json[jsonKey] = value && value.toJSON();
      return json;
    }, {});
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return JSON.stringify(Struct.typesToMap(this.registry, (0, _classPrivateFieldLooseBase2.default)(this, _Types)[_Types]));
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    return JSON.stringify(this.toJSON());
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    // we have keyof S here, cast to string to make it compatible with isBare
    const entries = [...this.entries()];
    return (0, _util.u8aConcat)(...entries // eslint-disable-next-line @typescript-eslint/unbound-method
    .filter(([, value]) => (0, _util.isFunction)(value === null || value === void 0 ? void 0 : value.toU8a)).map(([key, value]) => value.toU8a(!isBare || (0, _util.isBoolean)(isBare) ? isBare : isBare[key])));
  }

}

exports.Struct = Struct;