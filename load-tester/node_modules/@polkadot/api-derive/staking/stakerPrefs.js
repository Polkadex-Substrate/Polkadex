"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports._stakerPrefs = _stakerPrefs;
exports.stakerPrefs = stakerPrefs;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function _stakerPrefs(instanceId, api) {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  return (0, _util.memo)(instanceId, (accountId, eras, _withActive) => api.query.staking.erasValidatorPrefs.multi(eras.map(era => [era, accountId])).pipe((0, _operators.map)(all => all.map((validatorPrefs, index) => ({
    era: eras[index],
    validatorPrefs
  })))));
}

function stakerPrefs(instanceId, api) {
  return (0, _util.memo)(instanceId, (accountId, withActive = false) => api.derive.staking.erasHistoric(withActive).pipe((0, _operators.switchMap)(eras => api.derive.staking._stakerPrefs(accountId, eras, withActive))));
}