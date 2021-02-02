"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.getModuleTypes = getModuleTypes;
exports.getSpecTypes = getSpecTypes;
exports.getSpecRpc = getSpecRpc;
exports.getSpecAlias = getSpecAlias;
exports.getUpgradeVersion = getUpgradeVersion;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _chain = _interopRequireDefault(require("./chain"));

var _modules = _interopRequireDefault(require("./modules"));

var _spec = _interopRequireDefault(require("./spec"));

var _upgrades = _interopRequireDefault(require("./upgrades"));

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

// flatten a VersionedType[] into a Record<string, string>

/** @internal */
function filterVersions(versions = [], specVersion) {
  return versions.filter(({
    minmax: [min, max]
  }) => ((0, _util.isUndefined)(min) || specVersion >= min) && ((0, _util.isUndefined)(max) || specVersion <= max)).reduce((result, {
    types
  }) => _objectSpread(_objectSpread({}, result), types), {});
}
/**
 * @description Get types for specific modules (metadata override)
 */


function getModuleTypes({
  knownTypes
}, section) {
  var _knownTypes$typesAlia;

  return _objectSpread(_objectSpread({}, _modules.default[section] || {}), ((_knownTypes$typesAlia = knownTypes.typesAlias) === null || _knownTypes$typesAlia === void 0 ? void 0 : _knownTypes$typesAlia[section]) || {});
}
/**
 * @description Based on the chain and runtimeVersion, get the applicable types (ready for registration)
 */


function getSpecTypes({
  knownTypes
}, chainName, specName, specVersion) {
  var _knownTypes$typesBund, _knownTypes$typesBund2, _knownTypes$typesBund3, _knownTypes$typesBund4, _knownTypes$typesBund5, _knownTypes$typesBund6, _knownTypes$typesSpec, _knownTypes$typesChai;

  const _chainName = chainName.toString();

  const _specName = specName.toString();

  const _specVersion = (0, _util.bnToBn)(specVersion).toNumber(); // The order here is always, based on -
  //   - spec then chain
  //   - typesBundle takes higher precedence
  //   - types is the final catch-all override


  return _objectSpread(_objectSpread(_objectSpread(_objectSpread(_objectSpread(_objectSpread(_objectSpread({}, filterVersions(_spec.default[_specName], _specVersion)), filterVersions(_chain.default[_chainName], _specVersion)), filterVersions((_knownTypes$typesBund = knownTypes.typesBundle) === null || _knownTypes$typesBund === void 0 ? void 0 : (_knownTypes$typesBund2 = _knownTypes$typesBund.spec) === null || _knownTypes$typesBund2 === void 0 ? void 0 : (_knownTypes$typesBund3 = _knownTypes$typesBund2[_specName]) === null || _knownTypes$typesBund3 === void 0 ? void 0 : _knownTypes$typesBund3.types, _specVersion)), filterVersions((_knownTypes$typesBund4 = knownTypes.typesBundle) === null || _knownTypes$typesBund4 === void 0 ? void 0 : (_knownTypes$typesBund5 = _knownTypes$typesBund4.chain) === null || _knownTypes$typesBund5 === void 0 ? void 0 : (_knownTypes$typesBund6 = _knownTypes$typesBund5[_chainName]) === null || _knownTypes$typesBund6 === void 0 ? void 0 : _knownTypes$typesBund6.types, _specVersion)), ((_knownTypes$typesSpec = knownTypes.typesSpec) === null || _knownTypes$typesSpec === void 0 ? void 0 : _knownTypes$typesSpec[_specName]) || {}), ((_knownTypes$typesChai = knownTypes.typesChain) === null || _knownTypes$typesChai === void 0 ? void 0 : _knownTypes$typesChai[_chainName]) || {}), knownTypes.types || {});
}
/**
 * @description Based on the chain and runtimeVersion, get the applicable rpc definitions (ready for registration)
 */


function getSpecRpc({
  knownTypes
}, chainName, specName) {
  var _knownTypes$typesBund7, _knownTypes$typesBund8, _knownTypes$typesBund9, _knownTypes$typesBund10, _knownTypes$typesBund11, _knownTypes$typesBund12;

  const _chainName = chainName.toString();

  const _specName = specName.toString();

  return _objectSpread(_objectSpread({}, ((_knownTypes$typesBund7 = knownTypes.typesBundle) === null || _knownTypes$typesBund7 === void 0 ? void 0 : (_knownTypes$typesBund8 = _knownTypes$typesBund7.spec) === null || _knownTypes$typesBund8 === void 0 ? void 0 : (_knownTypes$typesBund9 = _knownTypes$typesBund8[_specName]) === null || _knownTypes$typesBund9 === void 0 ? void 0 : _knownTypes$typesBund9.rpc) || {}), ((_knownTypes$typesBund10 = knownTypes.typesBundle) === null || _knownTypes$typesBund10 === void 0 ? void 0 : (_knownTypes$typesBund11 = _knownTypes$typesBund10.chain) === null || _knownTypes$typesBund11 === void 0 ? void 0 : (_knownTypes$typesBund12 = _knownTypes$typesBund11[_chainName]) === null || _knownTypes$typesBund12 === void 0 ? void 0 : _knownTypes$typesBund12.rpc) || {});
}
/**
 * @description Based on the chain and runtimeVersion, get the applicable alias definitions (ready for registration)
 */


function getSpecAlias({
  knownTypes
}, chainName, specName) {
  var _knownTypes$typesBund13, _knownTypes$typesBund14, _knownTypes$typesBund15, _knownTypes$typesBund16, _knownTypes$typesBund17, _knownTypes$typesBund18;

  const _chainName = chainName.toString();

  const _specName = specName.toString(); // as per versions, first spec, then chain then finally non-versioned


  return _objectSpread(_objectSpread(_objectSpread({}, ((_knownTypes$typesBund13 = knownTypes.typesBundle) === null || _knownTypes$typesBund13 === void 0 ? void 0 : (_knownTypes$typesBund14 = _knownTypes$typesBund13.spec) === null || _knownTypes$typesBund14 === void 0 ? void 0 : (_knownTypes$typesBund15 = _knownTypes$typesBund14[_specName]) === null || _knownTypes$typesBund15 === void 0 ? void 0 : _knownTypes$typesBund15.alias) || {}), ((_knownTypes$typesBund16 = knownTypes.typesBundle) === null || _knownTypes$typesBund16 === void 0 ? void 0 : (_knownTypes$typesBund17 = _knownTypes$typesBund16.chain) === null || _knownTypes$typesBund17 === void 0 ? void 0 : (_knownTypes$typesBund18 = _knownTypes$typesBund17[_chainName]) === null || _knownTypes$typesBund18 === void 0 ? void 0 : _knownTypes$typesBund18.alias) || {}), knownTypes.typesAlias || {});
}
/**
 * @description Returns a version record for known chains where upgrades are being tracked
 */


function getUpgradeVersion(genesisHash, blockNumber) {
  const known = _upgrades.default.find(u => genesisHash.eq(u.genesisHash));

  return known ? [known.versions.reduce((last, version) => {
    return blockNumber.gt(version.blockNumber) ? version : last;
  }, undefined), known.versions.find(version => blockNumber.lte(version.blockNumber))] : [undefined, undefined];
}