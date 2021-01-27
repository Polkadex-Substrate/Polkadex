"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.detectedCapabilities = detectedCapabilities;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function mapCapabilities([systemRefcount32, systemRefcountDual, stakingVersion]) {
  const types = {}; // AccountInfo

  if (systemRefcountDual && systemRefcountDual.isTrue) {
    types.AccountInfo = 'AccountInfoWithProviders';
  } else if (systemRefcount32 && systemRefcount32.isTrue) {
    types.AccountInfo = 'AccountInfoWithRefCount';
  } // ValidatorPrefs


  if (stakingVersion) {
    if (stakingVersion.index >= 4) {
      // v1 = index 0, V5 = index 4
      types.ValidatorPrefs = 'ValidatorPrefsWithBlocked';
    } else {
      types.ValidatorPrefs = 'ValidatorPrefsWithCommission';
    }
  }

  return types;
}
/**
 * @description Query the chain for the specific capabilities
 */


function detectedCapabilities(api, blockHash) {
  var _api$query$system, _api$query$system2, _api$query$staking;

  const all = [(_api$query$system = api.query.system) === null || _api$query$system === void 0 ? void 0 : _api$query$system.upgradedToU32RefCount, (_api$query$system2 = api.query.system) === null || _api$query$system2 === void 0 ? void 0 : _api$query$system2.upgradedToDualRefCount, (_api$query$staking = api.query.staking) === null || _api$query$staking === void 0 ? void 0 : _api$query$staking.storageVersion];
  const included = all.map(c => !!c);
  const filtered = all.filter((_, index) => included[index]);
  return (filtered.length ? blockHash ? (0, _xRxjs.combineLatest)(filtered.map(c => c.at(blockHash))) : api.queryMulti(filtered) : (0, _xRxjs.of)([])).pipe((0, _operators.map)(results => {
    let offset = -1;
    return mapCapabilities(included.map(isIncluded => isIncluded ? results[++offset] : null));
  }), (0, _operators.take)(1));
}