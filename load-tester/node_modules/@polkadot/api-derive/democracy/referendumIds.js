"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.referendumIds = referendumIds;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function referendumIds(instanceId, api) {
  return (0, _util.memo)(instanceId, () => {
    var _api$query$democracy;

    return (_api$query$democracy = api.query.democracy) !== null && _api$query$democracy !== void 0 && _api$query$democracy.lowestUnbaked ? api.queryMulti([api.query.democracy.lowestUnbaked, api.query.democracy.referendumCount]).pipe((0, _operators.map)(([first, total]) => total.gt(first) // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    ? [...Array(total.sub(first).toNumber())].map((_, i) => first.addn(i)) : [])) : (0, _xRxjs.of)([]);
  });
}