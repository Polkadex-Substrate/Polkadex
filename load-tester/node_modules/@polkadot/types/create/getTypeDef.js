"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.getTypeDef = getTypeDef;

var _util = require("@polkadot/util");

var _sanitize = require("./sanitize");

var _types = require("./types");

var _typeSplit = require("./typeSplit");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
const MAX_NESTED = 64; // decode an enum of either of the following forms
//  { _enum: ['A', 'B', 'C'] }
//  { _enum: { A: AccountId, B: Balance, C: u32 } }

function _decodeEnum(value, details, count) {
  value.info = _types.TypeDefInfo.Enum; // not as pretty, but remain compatible with oo7 for both struct and Array types

  value.sub = Array.isArray(details) ? details.map(name => ({
    info: _types.TypeDefInfo.Plain,
    name,
    type: 'Null'
  })) : Object.entries(details).map(([name, type]) => // eslint-disable-next-line @typescript-eslint/no-use-before-define
  getTypeDef(type || 'Null', {
    name
  }, count));
  return value;
} // decode a set of the form
//   { _set: { A: 0b0001, B: 0b0010, C: 0b0100 } }


function _decodeSet(value, details) {
  value.info = _types.TypeDefInfo.Set;
  value.length = details._bitLength;
  value.sub = Object.entries(details).filter(([name]) => !name.startsWith('_')).map(([name, index]) => ({
    index,
    info: _types.TypeDefInfo.Plain,
    name,
    type: name
  }));
  return value;
} // decode a struct, set or enum
// eslint-disable-next-line @typescript-eslint/no-unused-vars


function _decodeStruct(value, type, _, count) {
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const parsed = JSON.parse(type);
  const keys = Object.keys(parsed);

  if (keys.length === 1 && keys[0] === '_enum') {
    return _decodeEnum(value, parsed[keys[0]], count);
  } else if (keys.length === 1 && keys[0] === '_set') {
    return _decodeSet(value, parsed[keys[0]]);
  }

  value.alias = parsed._alias ? new Map(Object.entries(parsed._alias)) : undefined;
  value.sub = keys.filter(name => !['_alias'].includes(name)).map(name => // eslint-disable-next-line @typescript-eslint/no-use-before-define
  getTypeDef(parsed[name], {
    name
  }, count));
  return value;
} // decode a fixed vector, e.g. [u8;32]
// eslint-disable-next-line @typescript-eslint/no-unused-vars


function _decodeFixedVec(value, type, _, count) {
  const [vecType, strLength, displayName] = type.substr(1, type.length - 2).split(';');
  const length = parseInt(strLength.trim(), 10); // as a first round, only u8 via u8aFixed, we can add more support

  (0, _util.assert)(length <= 256, `${type}: Only support for [Type; <length>], where length <= 256`);
  value.displayName = displayName;
  value.length = length; // eslint-disable-next-line @typescript-eslint/no-use-before-define

  value.sub = getTypeDef(vecType, {}, count);
  return value;
} // decode a tuple


function _decodeTuple(value, _, subType, count) {
  value.sub = subType.length === 0 ? [] // eslint-disable-next-line @typescript-eslint/no-use-before-define
  : (0, _typeSplit.typeSplit)(subType).map(inner => getTypeDef(inner, {}, count));
  return value;
} // decode a Int/UInt<bitLength[, name]>
// eslint-disable-next-line @typescript-eslint/no-unused-vars


function _decodeAnyInt(value, type, _, clazz) {
  const [strLength, displayName] = type.substr(clazz.length + 1, type.length - clazz.length - 1 - 1).split(',');
  const length = parseInt(strLength.trim(), 10); // as a first round, only u8 via u8aFixed, we can add more support

  (0, _util.assert)(length <= 8192 && length % 8 === 0, `${type}: Only support for ${clazz}<bitLength>, where length <= 8192 and a power of 8, found ${length}`);
  value.displayName = displayName;
  value.length = length;
  return value;
}

function _decodeInt(value, type, subType) {
  return _decodeAnyInt(value, type, subType, 'Int');
}

function _decodeUInt(value, type, subType) {
  return _decodeAnyInt(value, type, subType, 'UInt');
} // eslint-disable-next-line @typescript-eslint/no-unused-vars


function _decodeDoNotConstruct(value, type, _) {
  const NAME_LENGTH = 'DoNotConstruct'.length;
  value.displayName = type.substr(NAME_LENGTH + 1, type.length - NAME_LENGTH - 1 - 1);
  return value;
}

function hasWrapper(type, [start, end]) {
  return type.substr(0, start.length) === start && type.substr(-1 * end.length) === end;
}

const nestedExtraction = [['[', ']', _types.TypeDefInfo.VecFixed, _decodeFixedVec], ['{', '}', _types.TypeDefInfo.Struct, _decodeStruct], ['(', ')', _types.TypeDefInfo.Tuple, _decodeTuple], // the inner for these are the same as tuple, multiple values
['BTreeMap<', '>', _types.TypeDefInfo.BTreeMap, _decodeTuple], ['HashMap<', '>', _types.TypeDefInfo.HashMap, _decodeTuple], ['Int<', '>', _types.TypeDefInfo.Int, _decodeInt], ['Result<', '>', _types.TypeDefInfo.Result, _decodeTuple], ['UInt<', '>', _types.TypeDefInfo.UInt, _decodeUInt], ['DoNotConstruct<', '>', _types.TypeDefInfo.DoNotConstruct, _decodeDoNotConstruct]];
const wrappedExtraction = [['BTreeSet<', '>', _types.TypeDefInfo.BTreeSet], ['Compact<', '>', _types.TypeDefInfo.Compact], ['Linkage<', '>', _types.TypeDefInfo.Linkage], ['Option<', '>', _types.TypeDefInfo.Option], ['Vec<', '>', _types.TypeDefInfo.Vec]];

function extractSubType(type, [start, end]) {
  return type.substr(start.length, type.length - start.length - end.length);
} // eslint-disable-next-line @typescript-eslint/ban-types


function getTypeDef(_type, {
  displayName,
  name
} = {}, count = 0) {
  // create the type via Type, allowing types to be sanitized
  const type = (0, _sanitize.sanitize)(_type);
  const value = {
    displayName,
    info: _types.TypeDefInfo.Plain,
    name,
    type
  };
  (0, _util.assert)(++count !== MAX_NESTED, 'getTypeDef: Maximum nested limit reached');
  const nested = nestedExtraction.find(nested => hasWrapper(type, nested));

  if (nested) {
    value.info = nested[2];
    return nested[3](value, type, extractSubType(type, nested), count);
  }

  const wrapped = wrappedExtraction.find(wrapped => hasWrapper(type, wrapped));

  if (wrapped) {
    value.info = wrapped[2];
    value.sub = getTypeDef(extractSubType(type, wrapped), {}, count);
  }

  return value;
}