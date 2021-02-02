"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.extractTypes = extractTypes;

var _getTypeDef = require("@polkadot/types/create/getTypeDef");

var _types = require("@polkadot/types/types");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
// we are attempting to avoid circular refs, hence the path import

/** @internal */
function extractTypes(types) {
  return types.map(type => {
    const decoded = (0, _getTypeDef.getTypeDef)(type);

    switch (decoded.info) {
      case _types.TypeDefInfo.Plain:
        return decoded.type;

      case _types.TypeDefInfo.BTreeSet:
      case _types.TypeDefInfo.Compact:
      case _types.TypeDefInfo.Option:
      case _types.TypeDefInfo.Vec:
      case _types.TypeDefInfo.VecFixed:
        return extractTypes([decoded.sub.type]);

      case _types.TypeDefInfo.BTreeMap:
      case _types.TypeDefInfo.HashMap:
      case _types.TypeDefInfo.Result:
      case _types.TypeDefInfo.Tuple:
        return extractTypes(decoded.sub.map(({
          type
        }) => type));

      default:
        throw new Error(`Unhandled: Unable to create and validate type from ${type}`);
    }
  });
}