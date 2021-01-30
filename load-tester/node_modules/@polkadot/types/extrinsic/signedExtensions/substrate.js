"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;

var _emptyCheck = require("./emptyCheck");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0
const CheckMortality = {
  extrinsic: {
    era: 'ExtrinsicEra'
  },
  payload: {
    blockHash: 'Hash'
  }
};
var _default = {
  ChargeTransactionPayment: {
    extrinsic: {
      tip: 'Compact<Balance>'
    },
    payload: {}
  },
  CheckBlockGasLimit: _emptyCheck.emptyCheck,
  CheckEra: CheckMortality,
  CheckGenesis: {
    extrinsic: {},
    payload: {
      genesisHash: 'Hash'
    }
  },
  CheckMortality,
  CheckNonce: {
    extrinsic: {
      nonce: 'Compact<Index>'
    },
    payload: {}
  },
  CheckSpecVersion: {
    extrinsic: {},
    payload: {
      specVersion: 'u32'
    }
  },
  CheckTxVersion: {
    extrinsic: {},
    payload: {
      transactionVersion: 'u32'
    }
  },
  CheckVersion: {
    extrinsic: {},
    payload: {
      specVersion: 'u32'
    }
  },
  CheckWeight: _emptyCheck.emptyCheck,
  LockStakingStatus: _emptyCheck.emptyCheck,
  ValidateEquivocationReport: _emptyCheck.emptyCheck
};
exports.default = _default;