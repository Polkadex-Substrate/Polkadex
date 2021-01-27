"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.stashes = stashes;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description Retrieve the list of all validator stashes
 */
function stashes(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.query.staking.validators.keys().pipe((0, _operators.map)(keys => keys.map(key => key.args[0]).filter(a => a))));
}