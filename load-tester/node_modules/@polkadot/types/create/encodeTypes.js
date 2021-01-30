"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.paramsNotation = paramsNotation;
exports.encodeTypeDef = encodeTypeDef;
exports.withTypeString = withTypeString;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _types = require("./types");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const stringIdentity = value => value.toString();

const INFO_WRAP = ['BTreeMap', 'BTreeSet', 'Compact', 'HashMap', 'Option', 'Result', 'Vec'];

function paramsNotation(outer, inner, transform = stringIdentity) {
  return `${outer}${inner ? `<${(Array.isArray(inner) ? inner : [inner]).map(transform).join(', ')}>` : ''}`;
}

function encodeWithParams(typeDef, outer) {
  const {
    info,
    sub
  } = typeDef;

  switch (info) {
    case _types.TypeDefInfo.BTreeMap:
    case _types.TypeDefInfo.BTreeSet:
    case _types.TypeDefInfo.Compact:
    case _types.TypeDefInfo.HashMap:
    case _types.TypeDefInfo.Linkage:
    case _types.TypeDefInfo.Option:
    case _types.TypeDefInfo.Result:
    case _types.TypeDefInfo.Vec:
      return paramsNotation(outer, sub, param => encodeTypeDef(param));
  }

  throw new Error(`Unable to encode ${JSON.stringify(typeDef)} with params`);
}

function encodeDoNotConstruct({
  displayName
}) {
  return `DoNotConstruct<${displayName || 'Unknown'}>`;
}

function encodeSubTypes(sub, asEnum) {
  const names = sub.map(({
    name
  }) => name);
  (0, _util.assert)(names.every(n => !!n), `Subtypes does not have consistent names, ${names.join(', ')}`);
  const inner = sub.reduce((result, type) => _objectSpread(_objectSpread({}, result), {}, {
    [type.name]: encodeTypeDef(type)
  }), {});
  return JSON.stringify(asEnum ? {
    _enum: inner
  } : inner);
}

function encodeEnum(typeDef) {
  (0, _util.assert)(typeDef.sub && Array.isArray(typeDef.sub), 'Unable to encode Enum type');
  const sub = typeDef.sub; // c-like enums have all Null entries
  // TODO We need to take the disciminant into account and auto-add empty entries

  return sub.every(({
    type
  }) => type === 'Null') ? JSON.stringify({
    _enum: sub.map(({
      name
    }, index) => `${name || `Empty${index}`}`)
  }) : encodeSubTypes(sub, true);
}

function encodeStruct(typeDef) {
  (0, _util.assert)(typeDef.sub && Array.isArray(typeDef.sub), 'Unable to encode Struct type');
  return encodeSubTypes(typeDef.sub);
}

function encodeTuple(typeDef) {
  (0, _util.assert)(typeDef.sub && Array.isArray(typeDef.sub), 'Unable to encode Tuple type');
  return `(${typeDef.sub.map(type => encodeTypeDef(type)).join(', ')})`;
}

function encodeUInt({
  length
}, type) {
  (0, _util.assert)((0, _util.isNumber)(length), 'Unable to encode VecFixed type');
  return `${type}<${length}>`;
}

function encodeVecFixed({
  length,
  sub
}) {
  (0, _util.assert)((0, _util.isNumber)(length) && !(0, _util.isUndefined)(sub) && !Array.isArray(sub), 'Unable to encode VecFixed type');
  return `[${sub.type};${length}]`;
} // We setup a record here to ensure we have comprehensive coverage (any item not covered will result
// in a compile-time error with the missing index)


const encoders = {
  [_types.TypeDefInfo.BTreeMap]: typeDef => encodeWithParams(typeDef, 'BTreeMap'),
  [_types.TypeDefInfo.BTreeSet]: typeDef => encodeWithParams(typeDef, 'BTreeSet'),
  [_types.TypeDefInfo.Compact]: typeDef => encodeWithParams(typeDef, 'Compact'),
  [_types.TypeDefInfo.DoNotConstruct]: typeDef => encodeDoNotConstruct(typeDef),
  [_types.TypeDefInfo.Enum]: typeDef => encodeEnum(typeDef),
  [_types.TypeDefInfo.HashMap]: typeDef => encodeWithParams(typeDef, 'HashMap'),
  [_types.TypeDefInfo.Int]: typeDef => encodeUInt(typeDef, 'Int'),
  [_types.TypeDefInfo.Linkage]: typeDef => encodeWithParams(typeDef, 'Linkage'),
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  [_types.TypeDefInfo.Null]: typeDef => 'Null',
  [_types.TypeDefInfo.Option]: typeDef => encodeWithParams(typeDef, 'Option'),
  [_types.TypeDefInfo.Plain]: typeDef => typeDef.displayName || typeDef.type,
  [_types.TypeDefInfo.Result]: typeDef => encodeWithParams(typeDef, 'Result'),
  [_types.TypeDefInfo.Set]: typeDef => typeDef.type,
  [_types.TypeDefInfo.Struct]: typeDef => encodeStruct(typeDef),
  [_types.TypeDefInfo.Tuple]: typeDef => encodeTuple(typeDef),
  [_types.TypeDefInfo.UInt]: typeDef => encodeUInt(typeDef, 'UInt'),
  [_types.TypeDefInfo.Vec]: typeDef => encodeWithParams(typeDef, 'Vec'),
  [_types.TypeDefInfo.VecFixed]: typeDef => encodeVecFixed(typeDef)
};

function encodeType(typeDef) {
  const encoder = encoders[typeDef.info];
  (0, _util.assert)(encoder, `Cannot encode type: ${JSON.stringify(typeDef)}`);
  return encoder(typeDef);
}

function encodeTypeDef(typeDef) {
  (0, _util.assert)(!(0, _util.isUndefined)(typeDef.info), `Invalid type definition with no instance info, ${JSON.stringify(typeDef)}`); // In the case of contracts we do have the unfortunate situation where the displayName would
  // refer to "Option" when it is an option. For these, string it out, only using when actually
  // not a top-level element to be used

  if (typeDef.displayName && !INFO_WRAP.some(i => typeDef.displayName === i)) {
    return typeDef.displayName;
  }

  return encodeType(typeDef);
}

function withTypeString(typeDef) {
  return _objectSpread(_objectSpread({}, typeDef), {}, {
    type: encodeType(typeDef)
  });
}