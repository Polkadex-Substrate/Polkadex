"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.RpcCoder = void 0;

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/rpc-provider authors & contributors
// SPDX-License-Identifier: Apache-2.0
function formatErrorData(data) {
  if ((0, _util.isUndefined)(data)) {
    return '';
  }

  const formatted = `: ${(0, _util.isString)(data) ? data.replace(/Error\("/g, '').replace(/\("/g, '(').replace(/"\)/g, ')').replace(/\(/g, ', ').replace(/\)/g, '') : JSON.stringify(data)}`; // We need some sort of cut-off here since these can be very large and
  // very nested, pick a number and trim the result display to it

  return formatted.length <= 256 ? formatted : `${formatted.substr(0, 255)}…`;
}
/** @internal */


var _id = (0, _classPrivateFieldLooseKey2.default)("id");

class RpcCoder {
  constructor() {
    Object.defineProperty(this, _id, {
      writable: true,
      value: 0
    });
  }

  decodeResponse(response) {
    (0, _util.assert)(response, 'Empty response object received');
    (0, _util.assert)(response.jsonrpc === '2.0', 'Invalid jsonrpc field in decoded object');
    const isSubscription = !(0, _util.isUndefined)(response.params) && !(0, _util.isUndefined)(response.method);
    (0, _util.assert)((0, _util.isNumber)(response.id) || isSubscription && ((0, _util.isNumber)(response.params.subscription) || (0, _util.isString)(response.params.subscription)), 'Invalid id field in decoded object');

    this._checkError(response.error);

    (0, _util.assert)(!(0, _util.isUndefined)(response.result) || isSubscription, 'No result found in JsonRpc response');

    if (isSubscription) {
      this._checkError(response.params.error);

      return response.params.result;
    }

    return response.result;
  }

  encodeJson(method, params) {
    return JSON.stringify(this.encodeObject(method, params));
  }

  encodeObject(method, params) {
    return {
      id: ++(0, _classPrivateFieldLooseBase2.default)(this, _id)[_id],
      jsonrpc: '2.0',
      method,
      params
    };
  }

  getId() {
    return (0, _classPrivateFieldLooseBase2.default)(this, _id)[_id];
  }

  _checkError(error) {
    if (error) {
      const {
        code,
        data,
        message
      } = error;
      throw new Error(`${code}: ${message}${formatErrorData(data)}`);
    }
  }

}

exports.RpcCoder = RpcCoder;