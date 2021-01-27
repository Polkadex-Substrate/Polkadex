"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports._erasPoints = _erasPoints;
exports.erasPoints = erasPoints;

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

var _util3 = require("./util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
const CACHE_KEY = 'eraPoints';

function mapValidators({
  individual
}) {
  return [...individual.entries()].filter(([, points]) => points.gt(_util.BN_ZERO)).reduce((result, [validatorId, points]) => {
    result[validatorId.toString()] = points;
    return result;
  }, {});
}

function mapPoints(eras, points) {
  return eras.map((era, index) => ({
    era,
    eraPoints: points[index].total,
    validators: mapValidators(points[index])
  }));
}

function _erasPoints(instanceId, api) {
  return (0, _util2.memo)(instanceId, (eras, withActive) => {
    if (!eras.length) {
      return (0, _xRxjs.of)([]);
    }

    const cached = withActive ? [] : eras.map(era => _util2.deriveCache.get(`${CACHE_KEY}-${era.toString()}`)).filter(value => !!value);
    const remaining = (0, _util3.filterEras)(eras, cached);
    return !remaining.length ? (0, _xRxjs.of)(cached) : api.query.staking.erasRewardPoints.multi(remaining).pipe((0, _operators.map)(points => {
      const query = mapPoints(remaining, points);
      !withActive && query.forEach(q => _util2.deriveCache.set(`${CACHE_KEY}-${q.era.toString()}`, q));
      return eras.map(era => cached.find(cached => era.eq(cached.era)) || query.find(query => era.eq(query.era)));
    }));
  });
}

function erasPoints(instanceId, api) {
  return (0, _util2.memo)(instanceId, (withActive = false) => api.derive.staking.erasHistoric(withActive).pipe((0, _operators.switchMap)(eras => api.derive.staking._erasPoints(eras, withActive))));
}