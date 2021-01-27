"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decorateExtrinsics = decorateExtrinsics;

var _util = require("@polkadot/util");

var _createUnchecked = require("./createUnchecked");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decorateExtrinsics(registry, {
  modules
}, metaVersion) {
  return modules.filter(({
    calls
  }) => calls.isSome).reduce((result, {
    calls,
    index,
    name
  }, _sectionIndex) => {
    const sectionIndex = metaVersion >= 12 ? index.toNumber() : _sectionIndex;
    const section = (0, _util.stringCamelCase)(name);
    result[section] = calls.unwrap().reduce((newModule, callMetadata, methodIndex) => {
      newModule[(0, _util.stringCamelCase)(callMetadata.name)] = (0, _createUnchecked.createUnchecked)(registry, section, new Uint8Array([sectionIndex, methodIndex]), callMetadata);
      return newModule;
    }, {});
    return result;
  }, {});
}