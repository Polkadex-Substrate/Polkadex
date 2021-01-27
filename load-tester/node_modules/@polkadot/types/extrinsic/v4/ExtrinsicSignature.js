"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.GenericExtrinsicSignatureV4 = void 0;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

var _Enum = require("../../codec/Enum");

var _Struct = require("../../codec/Struct");

var _constants = require("../constants");

var _ExtrinsicPayload = require("./ExtrinsicPayload");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const FAKE_NONE = new Uint8Array();
const FAKE_SOME = new Uint8Array([1]);
/**
 * @name GenericExtrinsicSignatureV4
 * @description
 * A container for the [[Signature]] associated with a specific [[Extrinsic]]
 */

var _fakePrefix = (0, _classPrivateFieldLooseKey2.default)("fakePrefix");

class GenericExtrinsicSignatureV4 extends _Struct.Struct {
  constructor(registry, value, {
    isSigned
  } = {}) {
    super(registry, _objectSpread({
      signer: 'Address',
      // eslint-disable-next-line sort-keys
      signature: 'ExtrinsicSignature'
    }, registry.getSignedExtensionTypes()), GenericExtrinsicSignatureV4.decodeExtrinsicSignature(value, isSigned));
    Object.defineProperty(this, _fakePrefix, {
      writable: true,
      value: void 0
    });
    (0, _classPrivateFieldLooseBase2.default)(this, _fakePrefix)[_fakePrefix] = registry.createType('ExtrinsicSignature') instanceof _Enum.Enum ? FAKE_SOME : FAKE_NONE;
  }
  /** @internal */


  static decodeExtrinsicSignature(value, isSigned = false) {
    if (!value) {
      return _constants.EMPTY_U8A;
    } else if (value instanceof GenericExtrinsicSignatureV4) {
      return value;
    }

    return isSigned ? value : _constants.EMPTY_U8A;
  }
  /**
   * @description The length of the value when encoded as a Uint8Array
   */


  get encodedLength() {
    return this.isSigned ? super.encodedLength : 0;
  }
  /**
   * @description `true` if the signature is valid
   */


  get isSigned() {
    return !this.signature.isEmpty;
  }
  /**
   * @description The [[ExtrinsicEra]] (mortal or immortal) this signature applies to
   */


  get era() {
    return this.get('era');
  }
  /**
   * @description The [[Index]] for the signature
   */


  get nonce() {
    return this.get('nonce');
  }
  /**
   * @description The actual [[EcdsaSignature]], [[Ed25519Signature]] or [[Sr25519Signature]]
   */


  get signature() {
    // the second case here is when we don't have an enum signature, treat as raw
    return this.multiSignature.value || this.multiSignature;
  }
  /**
   * @description The raw [[ExtrinsicSignature]]
   */


  get multiSignature() {
    return this.get('signature');
  }
  /**
   * @description The [[Address]] that signed
   */


  get signer() {
    return this.get('signer');
  }
  /**
   * @description The [[Balance]] tip
   */


  get tip() {
    return this.get('tip');
  }

  _injectSignature(signer, signature, {
    era,
    nonce,
    tip
  }) {
    this.set('era', era);
    this.set('nonce', nonce);
    this.set('signer', signer);
    this.set('signature', signature);
    this.set('tip', tip);
    return this;
  }
  /**
   * @description Adds a raw signature
   */


  addSignature(signer, signature, payload) {
    return this._injectSignature(this.registry.createType('Address', signer), this.registry.createType('ExtrinsicSignature', signature), new _ExtrinsicPayload.GenericExtrinsicPayloadV4(this.registry, payload));
  }
  /**
   * @description Creates a payload from the supplied options
   */


  createPayload(method, {
    blockHash,
    era,
    genesisHash,
    nonce,
    runtimeVersion: {
      specVersion,
      transactionVersion
    },
    tip
  }) {
    return new _ExtrinsicPayload.GenericExtrinsicPayloadV4(this.registry, {
      blockHash,
      era: era || _constants.IMMORTAL_ERA,
      genesisHash,
      method: method.toHex(),
      nonce,
      specVersion,
      tip: tip || 0,
      transactionVersion: transactionVersion || 0
    });
  }
  /**
   * @description Generate a payload and applies the signature from a keypair
   */


  sign(method, account, options) {
    const signer = this.registry.createType('Address', account.addressRaw);
    const payload = this.createPayload(method, options);
    const signature = this.registry.createType('ExtrinsicSignature', payload.sign(account));
    return this._injectSignature(signer, signature, payload);
  }
  /**
   * @description Generate a payload and applies a fake signature
   */


  signFake(method, address, options) {
    const signer = this.registry.createType('Address', address);
    const payload = this.createPayload(method, options);
    const signature = this.registry.createType('ExtrinsicSignature', (0, _util.u8aConcat)((0, _classPrivateFieldLooseBase2.default)(this, _fakePrefix)[_fakePrefix], new Uint8Array(64).fill(0x42)));
    return this._injectSignature(signer, signature, payload);
  }
  /**
   * @description Encodes the value as a Uint8Array as per the SCALE specifications
   * @param isBare true when the value has none of the type-specific prefixes (internal)
   */


  toU8a(isBare) {
    return this.isSigned ? super.toU8a(isBare) : _constants.EMPTY_U8A;
  }

}

exports.GenericExtrinsicSignatureV4 = GenericExtrinsicSignatureV4;