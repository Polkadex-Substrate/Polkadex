"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.info = info;

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function retrieveNick(api, accountId) {
  var _api$query$nicks;

  return (accountId && (_api$query$nicks = api.query.nicks) !== null && _api$query$nicks !== void 0 && _api$query$nicks.nameOf ? api.query.nicks.nameOf(accountId) : (0, _xRxjs.of)(undefined)).pipe((0, _operators.map)(nameOf => nameOf !== null && nameOf !== void 0 && nameOf.isSome ? (0, _util.u8aToString)(nameOf.unwrap()[0]).substr(0, api.consts.nicks.maxLength.toNumber()) : undefined));
}
/**
 * @name info
 * @description Returns aux. info with regards to an account, current that includes the accountId, accountIndex and nickname
 */


function info(instanceId, api) {
  return (0, _util2.memo)(instanceId, address => api.derive.accounts.idAndIndex(address).pipe((0, _operators.switchMap)(([accountId, accountIndex]) => (0, _xRxjs.combineLatest)([(0, _xRxjs.of)({
    accountId,
    accountIndex
  }), api.derive.accounts.identity(accountId), retrieveNick(api, accountId)])), (0, _operators.map)(([{
    accountId,
    accountIndex
  }, identity, nickname]) => ({
    accountId,
    accountIndex,
    identity,
    nickname
  }))));
}