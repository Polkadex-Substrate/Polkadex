"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decorateErrors = decorateErrors;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
function isError({
  error,
  index
}, sectionIndex, errorIndex) {
  return index.eq(sectionIndex) && error.eq(errorIndex);
}
/** @internal */


function decorateErrors(_, {
  modules
}, metaVersion) {
  return modules.reduce((result, {
    errors,
    index,
    name
  }, _sectionIndex) => {
    if (!errors.length) {
      return result;
    }

    const sectionIndex = metaVersion >= 12 ? index.toNumber() : _sectionIndex;
    result[(0, _util.stringCamelCase)(name)] = errors.reduce((newModule, meta, errorIndex) => {
      // we don't camelCase the error name
      newModule[meta.name.toString()] = {
        is: moduleError => isError(moduleError, sectionIndex, errorIndex),
        meta
      };
      return newModule;
    }, {});
    return result;
  }, {});
}