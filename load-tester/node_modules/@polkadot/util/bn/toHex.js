"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bnToHex = bnToHex;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _number = require("../is/number");

var _u8a = require("../u8a");

var _toU8a = require("./toU8a");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const ZERO_STR = '0x00';

function bnToHex(value, arg1 = {
  bitLength: -1,
  isLe: false,
  isNegative: false
}, arg2) {
  if (!value) {
    return ZERO_STR;
  }

  const _options = _objectSpread({
    isLe: false,
    isNegative: false
  }, (0, _number.isNumber)(arg1) ? {
    bitLength: arg1,
    isLe: arg2
  } : arg1);

  return (0, _u8a.u8aToHex)((0, _toU8a.bnToU8a)(value, _options));
}