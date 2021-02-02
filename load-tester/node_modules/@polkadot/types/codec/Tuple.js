"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Tuple = void 0;

var _util = require("@polkadot/util");

var _AbstractArray = require("./AbstractArray");

var _utils = require("./utils");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decodeTuple(registry, _Types, value) {
  if ((0, _util.isU8a)(value) || (0, _util.isHex)(value)) {
    return (0, _utils.decodeU8a)(registry, (0, _util.u8aToU8a)(value), _Types);
  }

  const Types = Array.isArray(_Types) ? _Types : Object.values(_Types);
  return Types.map((Type, index) => {
    try {
      const entry = value === null || value === void 0 ? void 0 : value[index];

      if (entry instanceof Type) {
        return entry;
      }

      return new Type(registry, entry);
    } catch (error) {
      throw new Error(`Tuple: failed on ${index}:: ${error.message}`);
    }
  });
}
/**
 * @name Tuple
 * @description
 * A Tuple defines an anonymous fixed-length array, where each element has its
 * own type. It extends the base JS `Array` object.
 */


class Tuple extends _AbstractArray.AbstractArray {
  constructor(registry, Types, value) {
    const Clazzes = Array.isArray(Types) ? Types.map(type => (0, _utils.typeToConstructor)(registry, type)) : (0, _utils.mapToTypeMap)(registry, Types);
    super(registry, ...decodeTuple(registry, Clazzes, value));
    this._Types = void 0;
    this._Types = Clazzes;
  }

  static with(Types) {
    return class extends Tuple {
      constructor(registry, value) {
        super(registry, Types, value);
      }

    };
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    return this.reduce((length, entry) => {
      length += entry.encodedLength;
      return length;
    }, 0);
  }
  /**
   * @description The types definition of the tuple
   */


  get Types() {
    return Array.isArray(this._Types) ? this._Types.map(Type => new Type(this.registry).toRawType()) : Object.keys(this._Types);
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    const types = (Array.isArray(this._Types) ? this._Types : Object.values(this._Types)).map(Type => this.registry.getClassName(Type) || new Type(this.registry).toRawType());
    return `(${types.join(',')})`;
  }
  /**
   * @description Returns the string representation of the value
   */


  toString() {
    // Overwrite the default toString representation of Array.
    return JSON.stringify(this.toJSON());
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    return (0, _util.u8aConcat)(...this.map(entry => entry.toU8a(isBare)));
  }

}

exports.Tuple = Tuple;