"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.info = info;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function sortAccounts([, balanceA], [, balanceB]) {
  return balanceB.cmp(balanceA);
}

function queryElections(api) {
  const section = api.query.electionsPhragmen ? 'electionsPhragmen' : 'elections';
  return api.queryMulti([api.query.council.members, api.query[section].candidates, api.query[section].members, api.query[section].runnersUp]).pipe((0, _operators.map)(([councilMembers, candidates, members, runnersUp]) => ({
    candidacyBond: api.consts[section].candidacyBond,
    candidateCount: api.registry.createType('u32', candidates.length),
    candidates,
    desiredRunnersUp: api.consts[section].desiredRunnersUp,
    desiredSeats: api.consts[section].desiredMembers,
    members: members.length ? members.sort(sortAccounts) : councilMembers.map(accountId => [accountId, api.registry.createType('Balance')]),
    runnersUp: runnersUp.sort(sortAccounts),
    termDuration: api.consts[section].termDuration,
    votingBond: api.consts[section].votingBond
  })));
}
/**
 * @name info
 * @returns An object containing the combined results of the storage queries for
 * all relevant election module properties.
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.elections.info(({ members, candidates }) => {
 *   console.log(`There are currently ${members.length} council members and ${candidates.length} prospective council candidates.`);
 * });
 * ```
 */


function info(instanceId, api) {
  return (0, _util.memo)(instanceId, () => queryElections(api));
}