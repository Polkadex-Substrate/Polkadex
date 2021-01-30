"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.accounts = accounts;
exports.account = account;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _bn = _interopRequireDefault(require("bn.js"));

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const QUERY_OPTS = {
  withDestination: true,
  withLedger: true,
  withNominations: true,
  withPrefs: true
};

function groupByEra(list) {
  return list.reduce((map, {
    era,
    value
  }) => {
    const key = era.toString();
    map[key] = (map[key] || _util.BN_ZERO).add(value.unwrap());
    return map;
  }, {});
}

function calculateUnlocking(api, stakingLedger, sessionInfo) {
  const results = Object.entries(groupByEra(((stakingLedger === null || stakingLedger === void 0 ? void 0 : stakingLedger.unlocking) || []).filter(({
    era
  }) => era.unwrap().gt(sessionInfo.activeEra)))).map(([eraString, value]) => ({
    remainingEras: new _bn.default(eraString).isub(sessionInfo.activeEra),
    value: api.registry.createType('Balance', value)
  }));
  return results.length ? results : undefined;
}

function redeemableSum(api, stakingLedger, sessionInfo) {
  return api.registry.createType('Balance', ((stakingLedger === null || stakingLedger === void 0 ? void 0 : stakingLedger.unlocking) || []).reduce((total, {
    era,
    value
  }) => {
    return sessionInfo.activeEra.gte(era.unwrap()) ? total.iadd(value.unwrap()) : total;
  }, new _bn.default(0)));
}

function parseResult(api, sessionInfo, keys, query) {
  return _objectSpread(_objectSpread(_objectSpread({}, keys), query), {}, {
    redeemable: redeemableSum(api, query.stakingLedger, sessionInfo),
    unlocking: calculateUnlocking(api, query.stakingLedger, sessionInfo)
  });
}
/**
 * @description From a list of stashes, fill in all the relevant staking details
 */


function accounts(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountIds => api.derive.session.info().pipe((0, _operators.switchMap)(sessionInfo => (0, _xRxjs.combineLatest)([api.derive.staking.keysMulti(accountIds), api.derive.staking.queryMulti(accountIds, QUERY_OPTS)]).pipe((0, _operators.map)(([keys, queries]) => queries.map((query, index) => parseResult(api, sessionInfo, keys[index], query)))))));
}
/**
 * @description From a stash, retrieve the controllerId and fill in all the relevant staking details
 */


function account(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountId => api.derive.staking.accounts([accountId]).pipe((0, _operators.map)(([first]) => first)));
}