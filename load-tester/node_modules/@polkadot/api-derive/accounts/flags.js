"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.flags = flags;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function parseFlags(address, [electionsMembers, councilMembers, technicalCommitteeMembers, societyMembers, sudoKey]) {
  const isIncluded = id => address ? id.toString() === address.toString() : false;

  return {
    isCouncil: ((electionsMembers === null || electionsMembers === void 0 ? void 0 : electionsMembers.map(([id]) => id)) || councilMembers || []).some(isIncluded),
    isSociety: (societyMembers || []).some(isIncluded),
    isSudo: (sudoKey === null || sudoKey === void 0 ? void 0 : sudoKey.toString()) === (address === null || address === void 0 ? void 0 : address.toString()),
    isTechCommittee: (technicalCommitteeMembers || []).some(isIncluded)
  };
}
/**
 * @name info
 * @description Returns account membership flags
 */


function flags(instanceId, api) {
  return (0, _util.memo)(instanceId, address => {
    var _api$query$councilSec, _api$query$council, _api$query$technicalC, _api$query$society, _api$query$sudo;

    const councilSection = api.query.electionsPhragmen ? 'electionsPhragmen' : 'elections';
    return (0, _xRxjs.combineLatest)([address && (_api$query$councilSec = api.query[councilSection]) !== null && _api$query$councilSec !== void 0 && _api$query$councilSec.members ? api.query[councilSection].members() : (0, _xRxjs.of)(undefined), address && (_api$query$council = api.query.council) !== null && _api$query$council !== void 0 && _api$query$council.members ? api.query.council.members() : (0, _xRxjs.of)([]), address && (_api$query$technicalC = api.query.technicalCommittee) !== null && _api$query$technicalC !== void 0 && _api$query$technicalC.members ? api.query.technicalCommittee.members() : (0, _xRxjs.of)([]), address && (_api$query$society = api.query.society) !== null && _api$query$society !== void 0 && _api$query$society.members ? api.query.society.members() : (0, _xRxjs.of)([]), address && (_api$query$sudo = api.query.sudo) !== null && _api$query$sudo !== void 0 && _api$query$sudo.key ? api.query.sudo.key() : (0, _xRxjs.of)(undefined)]).pipe((0, _operators.map)(result => parseFlags(address, result)));
  });
}