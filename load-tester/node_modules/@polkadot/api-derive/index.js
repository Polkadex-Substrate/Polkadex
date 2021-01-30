"use strict";

var _interopRequireWildcard = require("@babel/runtime/helpers/interopRequireWildcard");

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  derive: true,
  decorateDerive: true
};
exports.decorateDerive = decorateDerive;
exports.derive = void 0;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var accounts = _interopRequireWildcard(require("./accounts"));

var balances = _interopRequireWildcard(require("./balances"));

var bounties = _interopRequireWildcard(require("./bounties"));

var chain = _interopRequireWildcard(require("./chain"));

var contracts = _interopRequireWildcard(require("./contracts"));

var council = _interopRequireWildcard(require("./council"));

var democracy = _interopRequireWildcard(require("./democracy"));

var elections = _interopRequireWildcard(require("./elections"));

var imOnline = _interopRequireWildcard(require("./imOnline"));

var parachains = _interopRequireWildcard(require("./parachains"));

var session = _interopRequireWildcard(require("./session"));

var society = _interopRequireWildcard(require("./society"));

var staking = _interopRequireWildcard(require("./staking"));

var technicalCommittee = _interopRequireWildcard(require("./technicalCommittee"));

var treasury = _interopRequireWildcard(require("./treasury"));

var tx = _interopRequireWildcard(require("./tx"));

var _type = require("./type");

Object.keys(_type).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _type[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _type[key];
    }
  });
});

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const derive = {
  accounts,
  balances,
  bounties,
  chain,
  contracts,
  council,
  democracy,
  elections,
  imOnline,
  parachains,
  session,
  society,
  staking,
  technicalCommittee,
  treasury,
  tx
};
exports.derive = derive;
// Enable derive only if some of these modules are available
const deriveAvail = {
  contracts: ['contracts'],
  council: ['council'],
  democracy: ['democracy'],
  elections: ['electionsPhragmen', 'elections'],
  imOnline: ['imOnline'],
  parachains: ['parachains', 'registrar'],
  session: ['session'],
  society: ['society'],
  staking: ['staking'],
  technicalCommittee: ['technicalCommittee'],
  treasury: ['treasury']
};
/**
 * Returns an object that will inject `api` into all the functions inside
 * `allSections`, and keep the object architecture of `allSections`.
 */

/** @internal */

function injectFunctions(instanceId, api, allSections) {
  const queryKeys = Object.keys(api.query);
  return Object.keys(allSections).filter(sectionName => !deriveAvail[sectionName] || deriveAvail[sectionName].some(query => queryKeys.includes(query))).reduce((deriveAcc, sectionName) => {
    const section = allSections[sectionName];
    deriveAcc[sectionName] = Object.keys(section).reduce((sectionAcc, _methodName) => {
      const methodName = _methodName; // Not sure what to do here, casting as any. Though the final types are good
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment,@typescript-eslint/no-unsafe-call

      const method = section[methodName](instanceId, api); // idem
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment,@typescript-eslint/no-unsafe-member-access

      sectionAcc[methodName] = method;
      return sectionAcc;
    }, {});
    return deriveAcc;
  }, {});
} // FIXME The return type of this function should be {...ExactDerive, ...DeriveCustom}
// For now we just drop the custom derive typings

/** @internal */


function decorateDerive(instanceId, api, custom = {}) {
  return _objectSpread(_objectSpread({}, injectFunctions(instanceId, api, derive)), injectFunctions(instanceId, api, custom));
}