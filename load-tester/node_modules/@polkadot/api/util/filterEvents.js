"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.filterEvents = filterEvents;

var _logging = require("./logging");

// Copyright 2017-2021 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
function filterEvents(extHash, {
  block: {
    extrinsics,
    header
  }
}, allEvents, status) {
  // extrinsics to hashes
  const myHash = extHash.toHex();
  const allHashes = extrinsics.map(ext => ext.hash.toHex()); // find the index of our extrinsic in the block

  const index = allHashes.indexOf(myHash); // if we do get the block after finalized, it _should_ be there

  if (index === -1) {
    // only warn on filtering with isInBlock (finalization finalizes after)
    if (status.isInBlock) {
      _logging.l.warn(`block ${header.hash.toHex()}: Unable to find extrinsic ${myHash} inside ${allHashes.join(', ')}`);
    }

    return;
  }

  return allEvents.filter(({
    phase
  }) => // only ApplyExtrinsic has the extrinsic index
  phase.isApplyExtrinsic && phase.asApplyExtrinsic.eqn(index));
}