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
const layout = {
  ContractCryptoHasher: {
    _enum: ['Blake2x256', 'Sha2x256', 'Keccak256']
  },
  ContractDiscriminant: 'u32',
  ContractLayoutArray: {
    offset: 'ContractLayoutKey',
    len: 'u32',
    cellsPerElem: 'u64',
    layout: 'ContractStorageLayout'
  },
  ContractLayoutCell: {
    key: 'ContractLayoutKey',
    ty: 'SiLookupTypeId'
  },
  ContractLayoutEnum: {
    dispatchKey: 'ContractLayoutKey',
    variants: 'BTreeMap<ContractDiscriminant, ContractLayoutStruct>'
  },
  ContractLayoutHash: {
    offset: 'ContractLayoutKey',
    strategy: 'ContractLayoutHashingStrategy',
    layout: 'ContractStorageLayout'
  },
  ContractLayoutHashingStrategy: {
    hasher: 'ContractCryptoHasher',
    postfix: 'Vec<u8>',
    prefix: 'Vec<u8>'
  },
  ContractLayoutKey: '[u8; 32]',
  ContractLayoutStruct: {
    fields: 'Vec<ContractLayoutStructField>'
  },
  ContractLayoutStructField: {
    layout: 'ContractStorageLayout',
    name: 'Text'
  },
  ContractStorageLayout: {
    _enum: {
      Cell: 'ContractLayoutCell',
      Hash: 'ContractLayoutHash',
      Array: 'ContractLayoutArray',
      Struct: 'ContractLayoutStruct',
      Enum: 'ContractLayoutEnum'
    }
  }
};
const spec = {
  ContractConstructorSpec: {
    name: 'Text',
    selector: 'ContractSelector',
    args: 'Vec<ContractMessageParamSpec>',
    docs: 'Vec<Text>'
  },
  ContractContractSpec: {
    constructors: 'Vec<ContractConstructorSpec>',
    messages: 'Vec<ContractMessageSpec>',
    events: 'Vec<ContractEventSpec>',
    docs: 'Vec<Text>'
  },
  ContractDisplayName: 'SiPath',
  ContractEventParamSpec: {
    name: 'Text',
    indexed: 'bool',
    type: 'ContractTypeSpec',
    docs: 'Vec<Text>'
  },
  ContractEventSpec: {
    name: 'Text',
    args: 'Vec<ContractEventParamSpec>',
    docs: 'Vec<Text>'
  },
  ContractMessageParamSpec: {
    name: 'Text',
    type: 'ContractTypeSpec'
  },
  ContractMessageSpec: {
    name: 'Text',
    selector: 'ContractSelector',
    mutates: 'bool',
    payable: 'bool',
    args: 'Vec<ContractMessageParamSpec>',
    returnType: 'Option<ContractTypeSpec>',
    docs: 'Vec<Text>'
  },
  ContractSelector: '[u8; 4]',
  ContractTypeSpec: {
    type: 'SiLookupTypeId',
    displayName: 'ContractDisplayName'
  }
};
var _default = {
  rpc: {},
  types: _objectSpread(_objectSpread(_objectSpread({}, layout), spec), {}, {
    ContractProject: {
      // added by ABI serialization
      metadataVersion: 'Text',
      source: 'ContractProjectSource',
      contract: 'ContractProjectContract',
      // expanded scale registry: RegistryReadOnly
      types: 'Vec<SiType>',
      // renamed from layout (ignored for now, incomplete)
      // storage: 'ContractStorageLayout',
      spec: 'ContractContractSpec'
    },
    ContractProjectContract: {
      name: 'Text',
      version: 'Text',
      authors: 'Vec<Text>',
      description: 'Option<Text>',
      documentation: 'Option<Text>',
      repository: 'Option<Text>',
      homepage: 'Option<Text>',
      license: 'Option<Text>'
    },
    ContractProjectSource: {
      hash: '[u8; 32]',
      language: 'Text',
      compiler: 'Text',
      wasm: 'Raw'
    }
  })
};
exports.default = _default;