"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.locks = locks;

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const LOCKUPS = [0, 1, 2, 4, 8, 16, 32];

function parseEnd(api, vote, {
  approved,
  end
}) {
  return [end, approved.isTrue && vote.isAye || approved.isFalse && vote.isNay ? end.add(api.consts.democracy.enactmentPeriod.muln(LOCKUPS[vote.conviction.index])) : _util.BN_ZERO];
}

function parseLock(api, [referendumId, accountVote], referendum) {
  const {
    balance,
    vote
  } = accountVote.asStandard;
  const [referendumEnd, unlockAt] = referendum.isFinished ? parseEnd(api, vote, referendum.asFinished) : [_util.BN_ZERO, _util.BN_ZERO];
  return {
    balance,
    isDelegated: false,
    isFinished: referendum.isFinished,
    referendumEnd,
    referendumId,
    unlockAt,
    vote
  };
}

function delegateLocks(api, {
  balance,
  conviction,
  target
}) {
  return api.derive.democracy.locks(target).pipe((0, _operators.map)(available => available.map(({
    isFinished,
    referendumEnd,
    referendumId,
    unlockAt,
    vote
  }) => ({
    balance,
    isDelegated: true,
    isFinished,
    referendumEnd,
    referendumId,
    unlockAt: unlockAt.isZero() ? unlockAt : referendumEnd.add(api.consts.democracy.enactmentPeriod.muln(LOCKUPS[conviction.index])),
    vote: api.registry.createType('Vote', {
      aye: vote.isAye,
      conviction
    })
  }))));
}

function directLocks(api, {
  votes
}) {
  if (!votes.length) {
    return (0, _xRxjs.of)([]);
  }

  return api.query.democracy.referendumInfoOf.multi(votes.map(([referendumId]) => referendumId)).pipe((0, _operators.map)(referendums => votes.map((vote, index) => [vote, referendums[index].unwrapOr(null)]).filter(item => !!item[1] && (0, _util.isUndefined)(item[1].end) && item[0][1].isStandard).map(([directVote, referendum]) => parseLock(api, directVote, referendum))));
}

function locks(instanceId, api) {
  return (0, _util2.memo)(instanceId, accountId => api.query.democracy.votingOf ? api.query.democracy.votingOf(accountId).pipe((0, _operators.switchMap)(voting => voting.isDirect ? directLocks(api, voting.asDirect) : voting.isDelegating ? delegateLocks(api, voting.asDelegating) : (0, _xRxjs.of)([]))) : (0, _xRxjs.of)([]));
}