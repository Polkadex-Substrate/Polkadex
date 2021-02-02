"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Combinator = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
var _allHasFired = (0, _classPrivateFieldLooseKey2.default)("allHasFired");

var _callback = (0, _classPrivateFieldLooseKey2.default)("callback");

var _fired = (0, _classPrivateFieldLooseKey2.default)("fired");

var _fns = (0, _classPrivateFieldLooseKey2.default)("fns");

var _isActive = (0, _classPrivateFieldLooseKey2.default)("isActive");

var _results = (0, _classPrivateFieldLooseKey2.default)("results");

var _subscriptions = (0, _classPrivateFieldLooseKey2.default)("subscriptions");

class Combinator {
  constructor(fns, callback) {
    Object.defineProperty(this, _allHasFired, {
      writable: true,
      value: false
    });
    Object.defineProperty(this, _callback, {
      writable: true,
      value: void 0
    });
    Object.defineProperty(this, _fired, {
      writable: true,
      value: []
    });
    Object.defineProperty(this, _fns, {
      writable: true,
      value: []
    });
    Object.defineProperty(this, _isActive, {
      writable: true,
      value: true
    });
    Object.defineProperty(this, _results, {
      writable: true,
      value: []
    });
    Object.defineProperty(this, _subscriptions, {
      writable: true,
      value: []
    });
    (0, _classPrivateFieldLooseBase2.default)(this, _callback)[_callback] = callback; // eslint-disable-next-line @typescript-eslint/require-await

    (0, _classPrivateFieldLooseBase2.default)(this, _subscriptions)[_subscriptions] = fns.map(async (input, index) => {
      const [fn, ...args] = Array.isArray(input) ? input : [input];

      (0, _classPrivateFieldLooseBase2.default)(this, _fired)[_fired].push(false);

      (0, _classPrivateFieldLooseBase2.default)(this, _fns)[_fns].push(fn); // Not quite 100% how to have a variable number at the front here
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/ban-types


      return fn(...args, this._createCallback(index));
    });
  }

  _allHasFired() {
    var _classPrivateFieldLoo;

    (_classPrivateFieldLoo = (0, _classPrivateFieldLooseBase2.default)(this, _allHasFired))[_allHasFired] || (_classPrivateFieldLoo[_allHasFired] = (0, _classPrivateFieldLooseBase2.default)(this, _fired)[_fired].filter(hasFired => !hasFired).length === 0);
    return (0, _classPrivateFieldLooseBase2.default)(this, _allHasFired)[_allHasFired];
  }

  _createCallback(index) {
    return value => {
      (0, _classPrivateFieldLooseBase2.default)(this, _fired)[_fired][index] = true;
      (0, _classPrivateFieldLooseBase2.default)(this, _results)[_results][index] = value;

      this._triggerUpdate();
    };
  }

  _triggerUpdate() {
    if (!(0, _classPrivateFieldLooseBase2.default)(this, _isActive)[_isActive] || !(0, _util.isFunction)((0, _classPrivateFieldLooseBase2.default)(this, _callback)[_callback]) || !this._allHasFired()) {
      return;
    }

    try {
      // eslint-disable-next-line @typescript-eslint/no-floating-promises
      (0, _classPrivateFieldLooseBase2.default)(this, _callback)[_callback]((0, _classPrivateFieldLooseBase2.default)(this, _results)[_results]);
    } catch (error) {// swallow, we don't want the handler to trip us up
    }
  }

  unsubscribe() {
    if (!(0, _classPrivateFieldLooseBase2.default)(this, _isActive)[_isActive]) {
      return;
    }

    (0, _classPrivateFieldLooseBase2.default)(this, _isActive)[_isActive] = false; // eslint-disable-next-line @typescript-eslint/no-misused-promises

    (0, _classPrivateFieldLooseBase2.default)(this, _subscriptions)[_subscriptions].forEach(async subscription => {
      try {
        const unsubscribe = await subscription;

        if ((0, _util.isFunction)(unsubscribe)) {
          unsubscribe();
        }
      } catch (error) {// ignore
      }
    });
  }

}

exports.Combinator = Combinator;