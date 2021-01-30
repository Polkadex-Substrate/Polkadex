"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createClass = createClass;
exports.ClassOfUnsafe = ClassOfUnsafe;
exports.ClassOf = ClassOf;
exports.getTypeClass = getTypeClass;

var _util = require("@polkadot/util");

var _codec = require("../codec");

var _primitive = require("../primitive");

var _getTypeDef = require("./getTypeDef");

var _types = require("./types");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
function createClass(registry, type) {
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  return getTypeClass(registry, (0, _getTypeDef.getTypeDef)(type));
} // An unsafe version of the `createType` below. It's unsafe because the `type`
// argument here can be any string, which, if it cannot be parsed, it will yield
// a runtime error.


function ClassOfUnsafe(registry, name) {
  return createClass(registry, name);
} // alias for createClass


function ClassOf(registry, name) {
  // TS2589: Type instantiation is excessively deep and possibly infinite.
  // The above happens with as Constructor<InterfaceTypes[K]>;
  // eslint-disable-next-line @typescript-eslint/no-unsafe-return
  return ClassOfUnsafe(registry, name);
}

function getSubDefArray(value) {
  (0, _util.assert)(value.sub && Array.isArray(value.sub), `Expected subtype as TypeDef[] in ${JSON.stringify(value)}`);
  return value.sub;
}

function getSubDef(value) {
  (0, _util.assert)(value.sub && !Array.isArray(value.sub), `Expected subtype as TypeDef in ${JSON.stringify(value)}`);
  return value.sub;
}

function getSubType(value) {
  return getSubDef(value).type;
} // create a maps of type string constructors from the input


function getTypeClassMap(value) {
  const result = {};
  return getSubDefArray(value).reduce((result, sub) => {
    result[sub.name] = sub.type;
    return result;
  }, result);
} // create an array of type string constructors from the input


function getTypeClassArray(value) {
  return getSubDefArray(value).map(({
    type
  }) => type);
}

function createInt({
  displayName,
  length
}, Clazz) {
  (0, _util.assert)((0, _util.isNumber)(length), `Expected bitLength information for ${displayName || Clazz.constructor.name}<bitLength>`);
  return Clazz.with(length, displayName);
}

function createHashMap(value, Clazz) {
  const [keyType, valueType] = getTypeClassArray(value);
  return Clazz.with(keyType, valueType);
}

const infoMapping = {
  [_types.TypeDefInfo.BTreeMap]: (registry, value) => createHashMap(value, _codec.BTreeMap),
  [_types.TypeDefInfo.BTreeSet]: (registry, value) => _codec.BTreeSet.with(getSubType(value)),
  [_types.TypeDefInfo.Compact]: (registry, value) => _codec.Compact.with(getSubType(value)),
  [_types.TypeDefInfo.DoNotConstruct]: (registry, value) => _primitive.DoNotConstruct.with(value.displayName),
  [_types.TypeDefInfo.Enum]: (registry, value) => _codec.Enum.with(getTypeClassMap(value)),
  [_types.TypeDefInfo.HashMap]: (registry, value) => createHashMap(value, _codec.HashMap),
  [_types.TypeDefInfo.Int]: (registry, value) => createInt(value, _codec.Int),
  // We have circular deps between Linkage & Struct
  [_types.TypeDefInfo.Linkage]: (registry, value) => {
    const type = `Option<${getSubType(value)}>`; // eslint-disable-next-line sort-keys

    const Clazz = _codec.Struct.with({
      previous: type,
      next: type
    }); // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access


    Clazz.prototype.toRawType = function () {
      // eslint-disable-next-line @typescript-eslint/restrict-template-expressions,@typescript-eslint/no-unsafe-member-access,@typescript-eslint/no-unsafe-call
      return `Linkage<${this.next.toRawType(true)}>`;
    };

    return Clazz;
  },
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  [_types.TypeDefInfo.Null]: (registry, _) => createClass(registry, 'Null'),
  [_types.TypeDefInfo.Option]: (registry, value) => _codec.Option.with(getSubType(value)),
  [_types.TypeDefInfo.Plain]: (registry, value) => registry.getOrUnknown(value.type),
  [_types.TypeDefInfo.Result]: (registry, value) => {
    const [Ok, Error] = getTypeClassArray(value); // eslint-disable-next-line @typescript-eslint/no-use-before-define

    return _codec.Result.with({
      Error,
      Ok
    });
  },
  [_types.TypeDefInfo.Set]: (registry, value) => {
    const result = {};
    return _codec.CodecSet.with(getSubDefArray(value).reduce((result, {
      index,
      name
    }) => {
      result[name] = index;
      return result;
    }, result), value.length);
  },
  [_types.TypeDefInfo.Struct]: (registry, value) => _codec.Struct.with(getTypeClassMap(value), value.alias),
  [_types.TypeDefInfo.Tuple]: (registry, value) => _codec.Tuple.with(getTypeClassArray(value)),
  [_types.TypeDefInfo.UInt]: (registry, value) => createInt(value, _codec.UInt),
  [_types.TypeDefInfo.Vec]: (registry, value) => {
    const subType = getSubType(value);
    return subType === 'u8' ? createClass(registry, 'Bytes') : _codec.Vec.with(subType);
  },
  [_types.TypeDefInfo.VecFixed]: (registry, {
    displayName,
    length,
    sub
  }) => {
    (0, _util.assert)((0, _util.isNumber)(length) && !(0, _util.isUndefined)(sub), 'Expected length & type information for fixed vector');
    return sub.type === 'u8' ? _codec.U8aFixed.with(length * 8, displayName) : _codec.VecFixed.with(sub.type, length);
  }
}; // Returns the type Class for construction

function getTypeClass(registry, value) {
  const Type = registry.get(value.type);

  if (Type) {
    return Type;
  }

  const getFn = infoMapping[value.info];
  (0, _util.assert)(getFn, `Unable to construct class from ${JSON.stringify(value)}`);
  return getFn(registry, value);
}