"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.memo = memo;

var _util = require("@polkadot/rpc-core/util");

var _util2 = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
// Wraps a derive, doing 2 things to optimize calls -
//   1. creates a memo of the inner fn -> Observable, removing when unsubscribed
//   2. wraps the observable in a drr() (which includes an unsub delay)

/** @internal */
function memo(instanceId, inner) {
  const cached = (0, _util2.memoize)((...params) => new _xRxjs.Observable(observer => {
    const subscription = inner(...params).subscribe(observer);
    return () => {
      cached.unmemoize(...params);
      subscription.unsubscribe();
    };
  }).pipe((0, _util.drr)()), {
    getInstanceId: () => instanceId
  });
  return cached;
}