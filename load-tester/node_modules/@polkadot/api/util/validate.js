"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.extractStorageArgs = extractStorageArgs;

var _util = require("@polkadot/util");

// Copyright 2017-2019 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
function sig({
  method,
  section
}, ...args) {
  return `${section}.${method}(${args.join(', ')})`;
}

function doDoubleMap(creator, args) {
  const {
    key1,
    key2
  } = creator.meta.type.asDoubleMap;
  (0, _util.assert)(args.length === 2, `${sig(creator, key1, key2)} is a doublemap, requiring 2 arguments, ${args.length} found`); // pass as tuple

  return [creator, args];
}

function doMap(creator, args) {
  const {
    key
  } = creator.meta.type.asMap;
  (0, _util.assert)(args.length === 1, `${sig(creator, key)} is a map, requiring 1 argument, ${args.length} found`); // expand

  return args.length ? [creator, args[0]] : [creator];
} // sets up the arguments in the form of [creator, args] ready to be used in a storage
// call. Additionally, it verifies that the correct number of arguments have been passed


function extractStorageArgs(creator, _args) {
  const args = _args.filter(arg => !(0, _util.isUndefined)(arg));

  if (creator.meta.type.isDoubleMap) {
    return doDoubleMap(creator, args);
  } else if (creator.meta.type.isMap) {
    return doMap(creator, args);
  }

  (0, _util.assert)(args.length === 0, `${sig(creator)} does not take any arguments, ${args.length} found`); // no args

  return [creator];
}