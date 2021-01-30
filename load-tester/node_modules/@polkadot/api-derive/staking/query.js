"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.query = query;
exports.queryMulti = queryMulti;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function parseDetails(stashId, controllerIdOpt, nominatorsOpt, rewardDestination, validatorPrefs, exposure, stakingLedgerOpt) {
  return {
    accountId: stashId,
    controllerId: controllerIdOpt && controllerIdOpt.unwrapOr(null),
    exposure,
    nominators: nominatorsOpt.isSome ? nominatorsOpt.unwrap().targets : [],
    rewardDestination,
    stakingLedger: stakingLedgerOpt.unwrapOrDefault(),
    stashId,
    validatorPrefs
  };
}

function getLedgers(api, optIds, {
  withLedger = false
}) {
  const ids = optIds.filter(opt => withLedger && !!opt && opt.isSome).map(opt => opt.unwrap());
  const emptyLed = api.registry.createType('Option<StakingLedger>');
  return (ids.length ? api.query.staking.ledger.multi(ids) : (0, _xRxjs.of)([])).pipe((0, _operators.map)(optLedgers => {
    let offset = -1;
    return optIds.map(opt => opt && opt.isSome ? optLedgers[++offset] || emptyLed : emptyLed);
  }));
}

function getStashInfo(api, stashIds, activeEra, {
  withController,
  withDestination,
  withExposure,
  withLedger,
  withNominations,
  withPrefs
}) {
  const emptyNoms = api.registry.createType('Option<Nominations>');
  const emptyRewa = api.registry.createType('RewardDestination');
  const emptyExpo = api.registry.createType('Exposure');
  const emptyPrefs = api.registry.createType('ValidatorPrefs');
  return (0, _xRxjs.combineLatest)([withController || withLedger ? api.query.staking.bonded.multi(stashIds) : (0, _xRxjs.of)(stashIds.map(() => null)), withNominations ? api.query.staking.nominators.multi(stashIds) : (0, _xRxjs.of)(stashIds.map(() => emptyNoms)), withDestination ? api.query.staking.payee.multi(stashIds) : (0, _xRxjs.of)(stashIds.map(() => emptyRewa)), withPrefs ? api.query.staking.validators.multi(stashIds) : (0, _xRxjs.of)(stashIds.map(() => emptyPrefs)), withExposure ? api.query.staking.erasStakers.multi(stashIds.map(stashId => [activeEra, stashId])) : (0, _xRxjs.of)(stashIds.map(() => emptyExpo))]);
}

function getBatch(api, activeEra, stashIds, flags) {
  return getStashInfo(api, stashIds, activeEra, flags).pipe((0, _operators.switchMap)(([controllerIdOpt, nominatorsOpt, rewardDestination, validatorPrefs, exposure]) => getLedgers(api, controllerIdOpt, flags).pipe((0, _operators.map)(stakingLedgerOpts => stashIds.map((stashId, index) => parseDetails(stashId, controllerIdOpt[index], nominatorsOpt[index], rewardDestination[index], validatorPrefs[index], exposure[index], stakingLedgerOpts[index]))))));
} //

/**
 * @description From a stash, retrieve the controllerId and all relevant details
 */


function query(instanceId, api) {
  return (0, _util.memo)(instanceId, (accountId, flags) => api.derive.staking.queryMulti([accountId], flags).pipe((0, _operators.map)(([first]) => first)));
}

function queryMulti(instanceId, api) {
  return (0, _util.memo)(instanceId, (accountIds, flags) => accountIds.length ? api.derive.session.indexes().pipe((0, _operators.switchMap)(({
    activeEra
  }) => {
    const stashIds = accountIds.map(accountId => api.registry.createType('AccountId', accountId));
    return getBatch(api, activeEra, stashIds, flags);
  })) : (0, _xRxjs.of)([]));
}