"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.erasHistoric = erasHistoric;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function erasHistoric(instanceId, api) {
  return (0, _util.memo)(instanceId, withActive => api.queryMulti([api.query.staking.activeEra, api.query.staking.historyDepth]).pipe((0, _operators.map)(([activeEraOpt, historyDepth]) => {
    const result = [];
    const max = historyDepth.toNumber();
    const activeEra = activeEraOpt.unwrapOrDefault().index;
    let lastEra = activeEra;

    while (lastEra.gten(0) && result.length < max) {
      if (lastEra !== activeEra || withActive === true) {
        result.push(api.registry.createType('EraIndex', lastEra));
      }

      lastEra = lastEra.subn(1);
    } // go from oldest to newest


    return result.reverse();
  })));
}