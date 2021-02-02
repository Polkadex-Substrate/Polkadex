"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.waitingInfo = waitingInfo;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const DEFAULT_FLAGS = {
  withController: true,
  withPrefs: true
};

function waitingInfo(instanceId, api) {
  return (0, _util.memo)(instanceId, (flags = DEFAULT_FLAGS) => (0, _xRxjs.combineLatest)([api.derive.staking.validators(), api.derive.staking.stashes()]).pipe((0, _operators.switchMap)(([{
    nextElected
  }, stashes]) => {
    const elected = nextElected.map(a => a.toString());
    const waiting = stashes.filter(v => !elected.includes(v.toString()));
    return api.derive.staking.queryMulti(waiting, flags).pipe((0, _operators.map)(info => ({
      info,
      waiting
    })));
  })));
}