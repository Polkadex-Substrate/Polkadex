"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createUnchecked = createUnchecked;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
function isTx(tx, callIndex) {
  return tx.callIndex[0] === callIndex[0] && tx.callIndex[1] === callIndex[1];
}
/** @internal */


function createUnchecked(registry, section, callIndex, callMetadata) {
  const expectedArgs = callMetadata.args;
  const funcName = (0, _util.stringCamelCase)(callMetadata.name);

  const extrinsicFn = (...args) => {
    (0, _util.assert)(expectedArgs.length === args.length, `Extrinsic ${section}.${funcName} expects ${expectedArgs.length.valueOf()} arguments, got ${args.length}.`);
    return registry.createType('Call', {
      args,
      callIndex
    }, callMetadata);
  };

  extrinsicFn.is = tx => isTx(tx, callIndex);

  extrinsicFn.callIndex = callIndex;
  extrinsicFn.meta = callMetadata;
  extrinsicFn.method = funcName;
  extrinsicFn.section = section;

  extrinsicFn.toJSON = () => callMetadata.toJSON();

  return extrinsicFn;
}