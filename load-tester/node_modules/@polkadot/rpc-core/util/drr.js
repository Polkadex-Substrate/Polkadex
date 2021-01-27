"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.drr = void 0;

var _util = require("@polkadot/util");

var _operators = require("@polkadot/x-rxjs/operators");

var _refCountDelay = require("./refCountDelay");

// Copyright 2017-2021 @polkadot/rpc-core authors & contributors
// SPDX-License-Identifier: Apache-2.0
const l = (0, _util.logger)('drr');

const CMP = (a, b) => JSON.stringify({
  t: a
}) === JSON.stringify({
  t: b
});

const ERR = error => {
  l.error(error);
  throw error;
};

const NOOP = () => undefined;
/**
 * Shorthand for distinctUntilChanged(), publishReplay(1) and refCount().
 *
 * @ignore
 * @internal
 */


const drr = ({
  delay,
  skipChange = false,
  skipTimeout = false
} = {}) => source$ => source$.pipe((0, _operators.catchError)(ERR), skipChange ? (0, _operators.tap)(NOOP) : (0, _operators.distinctUntilChanged)(CMP), (0, _operators.publishReplay)(1), skipTimeout ? (0, _operators.refCount)() : (0, _refCountDelay.refCountDelay)(delay));

exports.drr = drr;