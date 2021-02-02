"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.MetadataVersioned = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _codec = require("@polkadot/types/codec");

var _util = require("@polkadot/util");

var _toV = require("./v9/toV10");

var _toV2 = require("./v10/toV11");

var _toV3 = require("./v11/toV12");

var _toLatest = require("./v12/toLatest");

var _MagicNumber = require("./MagicNumber");

var _util2 = require("./util");

// Copyright 2017-2021 @polkadot/metadata authors & contributors
// SPDX-License-Identifier: Apache-2.0
var _converted = (0, _classPrivateFieldLooseKey2.default)("converted");

/**
 * @name MetadataVersioned
 * @description
 * The versioned runtime metadata as a decoded structure
 */
class MetadataVersioned extends _codec.Struct {
  constructor(registry, value) {
    super(registry, {
      magicNumber: _MagicNumber.MagicNumber,
      metadata: 'MetadataAll'
    }, value);
    Object.defineProperty(this, _converted, {
      writable: true,
      value: new Map()
    });
  }

  _assertVersion(version) {
    (0, _util.assert)(this.version <= version, `Cannot convert metadata from v${this.version} to v${version}`);
    return this.version === version;
  }

  _getVersion(version, fromPrev) {
    const asCurr = `asV${version}`;
    const asPrev = `asV${version - 1}`;

    if (this._assertVersion(version)) {
      return this._metadata[asCurr];
    }

    if (!(0, _classPrivateFieldLooseBase2.default)(this, _converted)[_converted].has(version)) {
      (0, _classPrivateFieldLooseBase2.default)(this, _converted)[_converted].set(version, fromPrev(this.registry, this[asPrev], this.version));
    }

    return (0, _classPrivateFieldLooseBase2.default)(this, _converted)[_converted].get(version);
  }
  /**
   * @description Returns the wrapped metadata as a limited calls-only (latest) version
   */


  get asCallsOnly() {
    return new MetadataVersioned(this.registry, {
      magicNumber: this.magicNumber,
      metadata: this.registry.createType('MetadataAll', (0, _util2.toCallsOnly)(this.registry, this.asLatest), this.version)
    });
  }
  /**
   * @description Returns the wrapped metadata as a V9 object
   */


  get asV9() {
    this._assertVersion(9);

    return this._metadata.asV9;
  }
  /**
   * @description Returns the wrapped values as a V10 object
   */


  get asV10() {
    return this._getVersion(10, _toV.toV10);
  }
  /**
   * @description Returns the wrapped values as a V11 object
   */


  get asV11() {
    return this._getVersion(11, _toV2.toV11);
  }
  /**
   * @description Returns the wrapped values as a V12 object
   */


  get asV12() {
    return this._getVersion(12, _toV3.toV12);
  }
  /**
   * @description Returns the wrapped values as a latest version object
   */


  get asLatest() {
    // This is non-existent & latest - applied here to do the module-specific type conversions
    return this._getVersion(13, _toLatest.toLatest);
  }
  /**
   * @description
   */


  get magicNumber() {
    return this.get('magicNumber');
  }
  /**
   * @description the metadata wrapped
   */


  get _metadata() {
    return this.get('metadata');
  }
  /**
   * @description the metadata version this structure represents
   */


  get version() {
    return this._metadata.index;
  }

  getUniqTypes(throwError) {
    return (0, _util2.getUniqTypes)(this.registry, this.asLatest, throwError);
  }

}

exports.MetadataVersioned = MetadataVersioned;