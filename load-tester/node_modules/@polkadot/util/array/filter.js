"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.arrayFilter = arrayFilter;

var _null = require("../is/null");

var _undefined = require("../is/undefined");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name arrayFilter
 * @summary Filters undefined and (optionally) null values from an array
 * @description
 * Returns a new array with all `undefined` values removed. Optionally, when `allowNulls = false`, it removes the `null` values as well
 * @example
 * <BR>
 *
 * ```javascript
 * import { arrayFilter } from '@polkadot/util';
 *
 * arrayFilter([0, void 0, true, null, false, '']); // [0, true, null, false, '']
 * arrayFilter([0, void 0, true, null, false, ''], false); // [0, true, false, '']
 * ```
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function arrayFilter(array, allowNulls = true) {
  return array.filter(value => !(0, _undefined.isUndefined)(value) && (allowNulls || !(0, _null.isNull)(value)));
}