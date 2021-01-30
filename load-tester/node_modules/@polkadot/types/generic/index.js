"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  GenericAccountId: true,
  GenericAccountIndex: true,
  GenericBlock: true,
  GenericCall: true,
  GenericChainProperties: true,
  GenericConsensusEngineId: true,
  GenericEvent: true,
  GenericEventData: true,
  GenericLookupSource: true,
  GenericMultiAddress: true,
  GenericVote: true
};
Object.defineProperty(exports, "GenericAccountId", {
  enumerable: true,
  get: function () {
    return _AccountId.GenericAccountId;
  }
});
Object.defineProperty(exports, "GenericAccountIndex", {
  enumerable: true,
  get: function () {
    return _AccountIndex.GenericAccountIndex;
  }
});
Object.defineProperty(exports, "GenericBlock", {
  enumerable: true,
  get: function () {
    return _Block.GenericBlock;
  }
});
Object.defineProperty(exports, "GenericCall", {
  enumerable: true,
  get: function () {
    return _Call.GenericCall;
  }
});
Object.defineProperty(exports, "GenericChainProperties", {
  enumerable: true,
  get: function () {
    return _ChainProperties.GenericChainProperties;
  }
});
Object.defineProperty(exports, "GenericConsensusEngineId", {
  enumerable: true,
  get: function () {
    return _ConsensusEngineId.GenericConsensusEngineId;
  }
});
Object.defineProperty(exports, "GenericEvent", {
  enumerable: true,
  get: function () {
    return _Event.GenericEvent;
  }
});
Object.defineProperty(exports, "GenericEventData", {
  enumerable: true,
  get: function () {
    return _Event.GenericEventData;
  }
});
Object.defineProperty(exports, "GenericLookupSource", {
  enumerable: true,
  get: function () {
    return _LookupSource.GenericLookupSource;
  }
});
Object.defineProperty(exports, "GenericMultiAddress", {
  enumerable: true,
  get: function () {
    return _MultiAddress.GenericMultiAddress;
  }
});
Object.defineProperty(exports, "GenericVote", {
  enumerable: true,
  get: function () {
    return _Vote.GenericVote;
  }
});

var _ethereum = require("../ethereum");

Object.keys(_ethereum).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _ethereum[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _ethereum[key];
    }
  });
});

var _AccountId = require("./AccountId");

var _AccountIndex = require("./AccountIndex");

var _Block = require("./Block");

var _Call = require("./Call");

var _ChainProperties = require("./ChainProperties");

var _ConsensusEngineId = require("./ConsensusEngineId");

var _Event = require("./Event");

var _LookupSource = require("./LookupSource");

var _MultiAddress = require("./MultiAddress");

var _Vote = require("./Vote");