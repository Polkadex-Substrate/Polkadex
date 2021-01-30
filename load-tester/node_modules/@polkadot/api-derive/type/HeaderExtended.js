"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.HeaderExtended = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _types = require("@polkadot/types");

var _definitions = _interopRequireDefault(require("@polkadot/types/interfaces/runtime/definitions"));

var _util = require("./util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
// We can ignore the properties, added via Struct.with
const _Header = _types.Struct.with(_definitions.default.types.Header);
/**
 * @name HeaderExtended
 * @description
 * A [[Block]] header with an additional `author` field that indicates the block author
 */


var _author = (0, _classPrivateFieldLooseKey2.default)("author");

var _validators = (0, _classPrivateFieldLooseKey2.default)("validators");

class HeaderExtended extends _Header {
  constructor(registry, header, validators) {
    super(registry, header);
    Object.defineProperty(this, _author, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _validators, {
      writable: true,
      value: void 0
    });
    (0, _classPrivateFieldLooseBase2.default)(this, _author)[_author] = (0, _util.extractAuthor)(this.digest, validators);
    (0, _classPrivateFieldLooseBase2.default)(this, _validators)[_validators] = validators;
  }
  /**
   * @description Convenience method, returns the author for the block
   */


  get author() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _author)[_author];
  }
  /**
   * @description Convenience method, returns the validators for the block
   */


  get validators() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _validators)[_validators];
  }

}

exports.HeaderExtended = HeaderExtended;