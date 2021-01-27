"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u8aSorted = u8aSorted;

var _undefined = require("../is/undefined");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function u8aSorted(u8as) {
  return u8as.sort((a, b) => {
    let i = 0;

    while (true) {
      if ((0, _undefined.isUndefined)(a[i]) && (0, _undefined.isUndefined)(b[i])) {
        return 0;
      } else if ((0, _undefined.isUndefined)(a[i])) {
        return -1;
      } else if ((0, _undefined.isUndefined)(b[i])) {
        return 1;
      }

      const cmp = a[i] - b[i];

      if (cmp !== 0) {
        return cmp;
      }

      i++;
    }
  });
}