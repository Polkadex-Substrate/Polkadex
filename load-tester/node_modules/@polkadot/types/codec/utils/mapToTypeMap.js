"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.mapToTypeMap = mapToTypeMap;

var _typeToConstructor = require("./typeToConstructor");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description takes an input map of the form `{ [string]: string | Constructor }` and returns a map of `{ [string]: Constructor }`
 */
function mapToTypeMap(registry, input) {
  return Object.entries(input).reduce((output, [key, type]) => {
    output[key] = (0, _typeToConstructor.typeToConstructor)(registry, type);
    return output;
  }, {});
}