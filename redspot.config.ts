import { RedspotUserConfig } from 'redspot/types';
import '@redspot/patract'; // import @redspot/patract plugin
import '@redspot/chai'; // import @redspot/chai plugin

import * as types from './typedef.json';

export default {
  defaultNetwork: 'development', // default network
  contract: {
    ink: {
      toolchain: 'nightly-2021-06-21', // specify the toolchain version for contract compliation
      sources: ['contracts/**/*'] // the directory where contracts locate
    }
  },
  networks: {
    // development network configuration
    development: {
      endpoint: 'ws://127.0.0.1:9944',
      types: types,
      gasLimit: '400000000000', // default gasLimit
      explorerUrl: 'https://polkadot.js.org/apps/#/explorer/query/?rpc=ws://127.0.0.1:9944/'
    }
  },
  mocha: {
    timeout: 60000
  }
} as RedspotUserConfig;