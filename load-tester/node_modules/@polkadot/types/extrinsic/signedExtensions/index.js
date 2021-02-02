"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.expandExtensionTypes = expandExtensionTypes;
exports.findUnknownExtensions = findUnknownExtensions;
exports.defaultExtensions = exports.allExtensions = void 0;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _polkadot = _interopRequireDefault(require("./polkadot"));

var _substrate = _interopRequireDefault(require("./substrate"));

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

// A mapping of the known signed extensions to the extra fields that they contain. Unlike in the actual extensions,
// we define the extra fields not as a Tuple, but rather as a struct so they can be named. These will be expanded
// into the various fields when added to the payload (we only support V4 onwards with these, V3 and earlier are
// deemed fixed and non-changeable)
const allExtensions = _objectSpread(_objectSpread({}, _substrate.default), _polkadot.default); // the v4 signed extensions (the order is important here, as applied by default)


exports.allExtensions = allExtensions;
const defaultExtensions = ['CheckVersion', 'CheckGenesis', 'CheckEra', 'CheckNonce', 'CheckWeight', 'ChargeTransactionPayment', 'CheckBlockGasLimit'];
exports.defaultExtensions = defaultExtensions;

function findUnknownExtensions(extensions, userExtensions = {}) {
  const names = [...Object.keys(allExtensions), ...Object.keys(userExtensions)];
  return extensions.filter(key => !names.includes(key));
}

function expandExtensionTypes(extensions, type, userExtensions = {}) {
  return extensions.map(key => allExtensions[key] || userExtensions[key]).filter(info => !!info).reduce((result, info) => _objectSpread(_objectSpread({}, result), info[type]), {});
}