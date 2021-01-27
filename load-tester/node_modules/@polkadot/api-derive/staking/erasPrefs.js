"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports._eraPrefs = _eraPrefs;
exports.eraPrefs = eraPrefs;
exports._erasPrefs = _erasPrefs;
exports.erasPrefs = erasPrefs;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const CACHE_KEY = 'eraPrefs';

function mapPrefs(era, all) {
  const validators = {};
  all.forEach(([key, prefs]) => {
    validators[key.args[1].toString()] = prefs;
  });
  return {
    era,
    validators
  };
}

function _eraPrefs(instanceId, api) {
  return (0, _util.memo)(instanceId, (era, withActive) => {
    const cacheKey = `${CACHE_KEY}-${era.toString()}`;
    const cached = withActive ? undefined : _util.deriveCache.get(cacheKey);
    return cached ? (0, _xRxjs.of)(cached) : api.query.staking.erasValidatorPrefs.entries(era).pipe((0, _operators.map)(prefs => {
      const value = mapPrefs(era, prefs);
      !withActive && _util.deriveCache.set(cacheKey, value);
      return value;
    }));
  });
}

function eraPrefs(instanceId, api) {
  return (0, _util.memo)(instanceId, era => api.derive.staking._eraPrefs(era, true));
}

function _erasPrefs(instanceId, api) {
  return (0, _util.memo)(instanceId, (eras, withActive) => eras.length ? (0, _xRxjs.combineLatest)(eras.map(era => api.derive.staking._eraPrefs(era, withActive))) : (0, _xRxjs.of)([]));
}

function erasPrefs(instanceId, api) {
  return (0, _util.memo)(instanceId, (withActive = false) => api.derive.staking.erasHistoric(withActive).pipe((0, _operators.switchMap)(eras => api.derive.staking._erasPrefs(eras, withActive))));
}