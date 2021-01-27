"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;
// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
// order important in structs... :)

/* eslint-disable sort-keys */
var _default = {
  rpc: {},
  types: {
    SiField: {
      name: 'Option<Text>',
      type: 'SiLookupTypeId'
    },
    SiLookupTypeId: 'u32',
    SiPath: 'Vec<Text>',
    SiType: {
      path: 'SiPath',
      params: 'Vec<SiLookupTypeId>',
      def: 'SiTypeDef'
    },
    SiTypeDef: {
      _enum: {
        Composite: 'SiTypeDefComposite',
        Variant: 'SiTypeDefVariant',
        Sequence: 'SiTypeDefSequence',
        Array: 'SiTypeDefArray',
        Tuple: 'SiTypeDefTuple',
        Primitive: 'SiTypeDefPrimitive'
      }
    },
    SiTypeDefArray: {
      len: 'u16',
      type: 'SiLookupTypeId'
    },
    SiTypeDefComposite: {
      fields: 'Vec<SiField>'
    },
    SiTypeDefVariant: {
      variants: 'Vec<SiVariant>'
    },
    SiTypeDefPrimitive: {
      _enum: ['Bool', 'Char', 'Str', 'U8', 'U16', 'U32', 'U64', 'U128', 'U256', 'I8', 'I16', 'I32', 'I64', 'I128', 'I256']
    },
    SiTypeDefSequence: {
      type: 'SiLookupTypeId'
    },
    SiTypeDefTuple: 'Vec<SiLookupTypeId>',
    SiVariant: {
      name: 'Text',
      fields: 'Vec<SiField>',
      discriminant: 'Option<u64>'
    }
  }
};
exports.default = _default;