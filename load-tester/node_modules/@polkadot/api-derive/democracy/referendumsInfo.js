"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports._referendumVotes = _referendumVotes;
exports._referendumsVotes = _referendumsVotes;
exports._referendumInfo = _referendumInfo;
exports.referendumsInfo = referendumsInfo;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

var _util3 = require("./util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

function votesPrev(api, referendumId) {
  return api.query.democracy.votersFor(referendumId).pipe((0, _operators.switchMap)(votersFor => (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(votersFor), votersFor.length ? api.query.democracy.voteOf.multi(votersFor.map(accountId => [referendumId, accountId])) : (0, _xRxjs.of)([]), api.derive.balances.votingBalances(votersFor)])), (0, _operators.map)(([votersFor, votes, balances]) => votersFor.map((accountId, index) => ({
    accountId,
    balance: balances[index].votingBalance || api.registry.createType('Balance'),
    isDelegating: false,
    vote: votes[index] || api.registry.createType('Vote')
  }))));
}

function extractVotes(mapped, referendumId) {
  return mapped.filter(([, voting]) => voting.isDirect).map(([accountId, voting]) => [accountId, voting.asDirect.votes.filter(([idx]) => idx.eq(referendumId))]).filter(([, directVotes]) => !!directVotes.length).reduce((result, [accountId, votes]) => // FIXME We are ignoring split votes
  votes.reduce((result, [, vote]) => {
    if (vote.isStandard) {
      result.push(_objectSpread({
        accountId,
        isDelegating: false
      }, vote.asStandard));
    }

    return result;
  }, result), []);
}

function votesCurr(api, referendumId) {
  return api.query.democracy.votingOf.entries().pipe((0, _operators.map)(allVoting => {
    const mapped = allVoting.map(([key, voting]) => [key.args[0], voting]);
    const votes = extractVotes(mapped, referendumId);
    const delegations = mapped.filter(([, voting]) => voting.isDelegating).map(([accountId, voting]) => [accountId, voting.asDelegating]); // add delegations

    delegations.forEach(([accountId, {
      balance,
      conviction,
      target
    }]) => {
      // Are we delegating to a delegator
      const toDelegator = delegations.find(([accountId]) => accountId.eq(target));
      const to = votes.find(({
        accountId
      }) => accountId.eq(toDelegator ? toDelegator[0] : target)); // this delegation has a target

      if (to) {
        votes.push({
          accountId,
          balance,
          isDelegating: true,
          vote: api.registry.createType('Vote', {
            aye: to.vote.isAye,
            conviction
          })
        });
      }
    });
    return votes;
  }));
}

function _referendumVotes(instanceId, api) {
  return (0, _util2.memo)(instanceId, referendum => (0, _xRxjs.combineLatest)([api.derive.democracy.sqrtElectorate(), (0, _util.isFunction)(api.query.democracy.votingOf) ? votesCurr(api, referendum.index) : votesPrev(api, referendum.index)]).pipe((0, _operators.map)(([sqrtElectorate, votes]) => (0, _util3.calcVotes)(sqrtElectorate, referendum, votes))));
}

function _referendumsVotes(instanceId, api) {
  return (0, _util2.memo)(instanceId, referendums => referendums.length ? (0, _xRxjs.combineLatest)(referendums.map(referendum => api.derive.democracy._referendumVotes(referendum))) : (0, _xRxjs.of)([]));
}

function _referendumInfo(instanceId, api) {
  return (0, _util2.memo)(instanceId, (index, info) => {
    const status = (0, _util3.getStatus)(info);
    return status ? api.query.democracy.preimages(status.proposalHash).pipe((0, _operators.map)(preimage => ({
      image: (0, _util3.parseImage)(api, preimage),
      imageHash: status.proposalHash,
      index: api.registry.createType('ReferendumIndex', index),
      status
    }))) : (0, _xRxjs.of)(null);
  });
}

function referendumsInfo(instanceId, api) {
  return (0, _util2.memo)(instanceId, ids => ids.length ? api.query.democracy.referendumInfoOf.multi(ids).pipe((0, _operators.switchMap)(infos => (0, _xRxjs.combineLatest)(ids.map((id, index) => api.derive.democracy._referendumInfo(id, infos[index])))), (0, _operators.map)(infos => infos.filter(referendum => !!referendum))) : (0, _xRxjs.of)([]));
}