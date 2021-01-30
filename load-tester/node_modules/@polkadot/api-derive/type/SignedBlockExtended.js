"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.SignedBlockExtended = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _types = require("@polkadot/types");

var _definitions = _interopRequireDefault(require("@polkadot/types/interfaces/runtime/definitions"));

var _util = require("./util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
// We can ignore the properties, added via Struct.with
const _SignedBlock = _types.Struct.with(_definitions.default.types.SignedBlock);

function mapExtrinsics(extrinsics, records) {
  return extrinsics.map((extrinsic, index) => {
    let dispatchError;
    let dispatchInfo;
    const events = records.filter(({
      phase
    }) => phase.isApplyExtrinsic && phase.asApplyExtrinsic.eq(index)).map(({
      event
    }) => {
      if (event.section === 'system') {
        if (event.method === 'ExtrinsicSuccess') {
          dispatchInfo = event.data[0];
        } else if (event.method === 'ExtrinsicFailed') {
          dispatchError = event.data[0];
          dispatchInfo = event.data[1];
        }
      }

      return event;
    });
    return {
      dispatchError,
      dispatchInfo,
      events,
      extrinsic
    };
  });
}
/**
 * @name SignedBlockExtended
 * @description
 * A [[Block]] header with an additional `author` field that indicates the block author
 */


var _author = (0, _classPrivateFieldLooseKey2.default)("author");

var _events = (0, _classPrivateFieldLooseKey2.default)("events");

var _extrinsics = (0, _classPrivateFieldLooseKey2.default)("extrinsics");

class SignedBlockExtended extends _SignedBlock {
  constructor(registry, block, events, sessionValidators) {
    super(registry, block);
    Object.defineProperty(this, _author, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _events, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _extrinsics, {
      writable: true,
      value: void 0
    });
    (0, _classPrivateFieldLooseBase2.default)(this, _author)[_author] = (0, _util.extractAuthor)(this.block.header.digest, sessionValidators);
    (0, _classPrivateFieldLooseBase2.default)(this, _events)[_events] = events || [];
    (0, _classPrivateFieldLooseBase2.default)(this, _extrinsics)[_extrinsics] = mapExtrinsics(this.block.extrinsics, (0, _classPrivateFieldLooseBase2.default)(this, _events)[_events]);
  }
  /**
   * @description Convenience method, returns the author for the block
   */


  get author() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _author)[_author];
  }
  /**
   * @description Convenience method, returns the events associated with the block
   */


  get events() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _events)[_events];
  }
  /**
   * @description Returns the extrinsics and their events, mapped
   */


  get extrinsics() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _extrinsics)[_extrinsics];
  }

}

exports.SignedBlockExtended = SignedBlockExtended;