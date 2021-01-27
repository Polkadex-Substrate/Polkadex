"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
var _exportNames = {
  GenericExtrinsic: true,
  GenericExtrinsicEra: true,
  GenericMortalEra: true,
  GenericImmortalEra: true,
  GenericExtrinsicPayload: true,
  GenericExtrinsicPayloadUnknown: true,
  GenericExtrinsicUnknown: true,
  GenericSignerPayload: true
};
Object.defineProperty(exports, "GenericExtrinsic", {
  enumerable: true,
  get: function () {
    return _Extrinsic.GenericExtrinsic;
  }
});
Object.defineProperty(exports, "GenericExtrinsicEra", {
  enumerable: true,
  get: function () {
    return _ExtrinsicEra.GenericExtrinsicEra;
  }
});
Object.defineProperty(exports, "GenericMortalEra", {
  enumerable: true,
  get: function () {
    return _ExtrinsicEra.MortalEra;
  }
});
Object.defineProperty(exports, "GenericImmortalEra", {
  enumerable: true,
  get: function () {
    return _ExtrinsicEra.ImmortalEra;
  }
});
Object.defineProperty(exports, "GenericExtrinsicPayload", {
  enumerable: true,
  get: function () {
    return _ExtrinsicPayload.GenericExtrinsicPayload;
  }
});
Object.defineProperty(exports, "GenericExtrinsicPayloadUnknown", {
  enumerable: true,
  get: function () {
    return _ExtrinsicPayloadUnknown.GenericExtrinsicPayloadUnknown;
  }
});
Object.defineProperty(exports, "GenericExtrinsicUnknown", {
  enumerable: true,
  get: function () {
    return _ExtrinsicUnknown.GenericExtrinsicUnknown;
  }
});
Object.defineProperty(exports, "GenericSignerPayload", {
  enumerable: true,
  get: function () {
    return _SignerPayload.GenericSignerPayload;
  }
});

var _Extrinsic = require("./Extrinsic");

var _ExtrinsicEra = require("./ExtrinsicEra");

var _ExtrinsicPayload = require("./ExtrinsicPayload");

var _ExtrinsicPayloadUnknown = require("./ExtrinsicPayloadUnknown");

var _ExtrinsicUnknown = require("./ExtrinsicUnknown");

var _SignerPayload = require("./SignerPayload");

var _v = require("./v4");

Object.keys(_v).forEach(function (key) {
  if (key === "default" || key === "__esModule") return;
  if (Object.prototype.hasOwnProperty.call(_exportNames, key)) return;
  if (key in exports && exports[key] === _v[key]) return;
  Object.defineProperty(exports, key, {
    enumerable: true,
    get: function () {
      return _v[key];
    }
  });
});