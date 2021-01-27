"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.Events = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _eventemitter2 = _interopRequireDefault(require("eventemitter3"));

// Copyright 2017-2021 @polkadot/api authors & contributors
// SPDX-License-Identifier: Apache-2.0
var _eventemitter = (0, _classPrivateFieldLooseKey2.default)("eventemitter");

class Events {
  constructor() {
    Object.defineProperty(this, _eventemitter, {
      writable: true,
      value: new _eventemitter2.default()
    });
  }

  emit(type, ...args) {
    return (0, _classPrivateFieldLooseBase2.default)(this, _eventemitter)[_eventemitter].emit(type, ...args);
  }
  /**
   * @description Attach an eventemitter handler to listen to a specific event
   *
   * @param type The type of event to listen to. Available events are `connected`, `disconnected`, `ready` and `error`
   * @param handler The callback to be called when the event fires. Depending on the event type, it could fire with additional arguments.
   *
   * @example
   * <BR>
   *
   * ```javascript
   * api.on('connected', (): void => {
   *   console.log('API has been connected to the endpoint');
   * });
   *
   * api.on('disconnected', (): void => {
   *   console.log('API has been disconnected from the endpoint');
   * });
   * ```
   */


  on(type, handler) {
    (0, _classPrivateFieldLooseBase2.default)(this, _eventemitter)[_eventemitter].on(type, handler);

    return this;
  }
  /**
   * @description Remove the given eventemitter handler
   *
   * @param type The type of event the callback was attached to. Available events are `connected`, `disconnected`, `ready` and `error`
   * @param handler The callback to unregister.
   *
   * @example
   * <BR>
   *
   * ```javascript
   * const handler = (): void => {
   *  console.log('Connected !);
   * };
   *
   * // Start listening
   * api.on('connected', handler);
   *
   * // Stop listening
   * api.off('connected', handler);
   * ```
   */


  off(type, handler) {
    (0, _classPrivateFieldLooseBase2.default)(this, _eventemitter)[_eventemitter].removeListener(type, handler);

    return this;
  }
  /**
   * @description Attach an one-time eventemitter handler to listen to a specific event
   *
   * @param type The type of event to listen to. Available events are `connected`, `disconnected`, `ready` and `error`
   * @param handler The callback to be called when the event fires. Depending on the event type, it could fire with additional arguments.
   *
   * @example
   * <BR>
   *
   * ```javascript
   * api.once('connected', (): void => {
   *   console.log('API has been connected to the endpoint');
   * });
   *
   * api.once('disconnected', (): void => {
   *   console.log('API has been disconnected from the endpoint');
   * });
   * ```
   */


  once(type, handler) {
    (0, _classPrivateFieldLooseBase2.default)(this, _eventemitter)[_eventemitter].once(type, handler);

    return this;
  }

}

exports.Events = Events;