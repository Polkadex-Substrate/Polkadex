"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decorateConstants = decorateConstants;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0

/** @internal */
function decorateConstants(registry, {
  modules
}) {
  return modules.reduce((result, {
    constants,
    name
  }) => {
    if (constants.isEmpty) {
      return result;
    } // For access, we change the index names, i.e. Democracy.EnactmentPeriod -> democracy.enactmentPeriod


    result[(0, _util.stringCamelCase)(name)] = constants.reduce((newModule, meta) => {
      // convert to the natural type as received
      const type = meta.type.toString();
      const codec = registry.createType(type, (0, _util.hexToU8a)(meta.value.toHex()));
      codec.meta = meta;
      newModule[(0, _util.stringCamelCase)(meta.name)] = codec;
      return newModule;
    }, {});
    return result;
  }, {});
}