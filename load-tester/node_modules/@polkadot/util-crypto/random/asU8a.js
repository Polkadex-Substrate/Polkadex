"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.randomAsU8a = randomAsU8a;

var _xRandomvalues = require("@polkadot/x-randomvalues");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name randomAsU8a
 * @summary Creates a Uint8Array filled with random bytes.
 * @description
 * Returns a `Uint8Array` with the specified (optional) length filled with random bytes.
 * @example
 * <BR>
 *
 * ```javascript
 * import { randomAsU8a } from '@polkadot/util-crypto';
 *
 * randomAsU8a(); // => Uint8Array([...])
 * ```
 */
function randomAsU8a(length = 32) {
  return (0, _xRandomvalues.getRandomValues)(new Uint8Array(length));
}