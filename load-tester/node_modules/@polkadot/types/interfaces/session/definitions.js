"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
// order important in structs... :)

/* eslint-disable sort-keys */
// The runtime definition of SessionKeys are passed as a Trait to session
// Defined in `node/runtime/src/lib.rs` as follow
//   impl_opaque_keys! {
//     pub struct SessionKeys {
// Here we revert to tuples to keep the interfaces "opaque", as per the use
const keyTypes = {
  // default to Substrate master defaults, 4 keys (polkadot master, 5 keys)
  Keys: 'SessionKeys4',
  // shortcuts for 1-9 key tuples
  SessionKeys1: '(AccountId)',
  SessionKeys2: '(AccountId, AccountId)',
  // older substrate master
  SessionKeys3: '(AccountId, AccountId, AccountId)',
  // CC2, Substrate master
  SessionKeys4: '(AccountId, AccountId, AccountId, AccountId)',
  // CC3
  SessionKeys5: '(AccountId, AccountId, AccountId, AccountId, AccountId)',
  SessionKeys6: '(AccountId, AccountId, AccountId, AccountId, AccountId, AccountId)',
  SessionKeys7: '(AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId)',
  SessionKeys8: '(AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId)',
  SessionKeys9: '(AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId, AccountId)'
};
var _default = {
  rpc: {},
  types: _objectSpread(_objectSpread({}, keyTypes), {}, {
    FullIdentification: 'Exposure',
    IdentificationTuple: '(ValidatorId, FullIdentification)',
    MembershipProof: {
      session: 'SessionIndex',
      trieNodes: 'Vec<Vec<u8>>',
      validatorCount: 'ValidatorCount'
    },
    SessionIndex: 'u32',
    ValidatorCount: 'u32'
  })
};
exports.default = _default;