"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createClass = createClass;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _classPrivateFieldLooseBase2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseBase"));

var _classPrivateFieldLooseKey2 = _interopRequireDefault(require("@babel/runtime/helpers/classPrivateFieldLooseKey"));

var _util = require("@polkadot/util");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

var _Result = require("./Result");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

const identity = input => input;

function createClass({
  api,
  apiType,
  decorateMethod
}) {
  // an instance of the base extrinsic for us to extend
  const ExtrinsicBase = api.registry.createClass('Extrinsic');

  var _ignoreStatusCb = (0, _classPrivateFieldLooseKey2.default)("ignoreStatusCb");

  var _transformResult = (0, _classPrivateFieldLooseKey2.default)("transformResult");

  var _makeEraOptions = (0, _classPrivateFieldLooseKey2.default)("makeEraOptions");

  var _makeSignOptions = (0, _classPrivateFieldLooseKey2.default)("makeSignOptions");

  var _makeSignAndSendOptions = (0, _classPrivateFieldLooseKey2.default)("makeSignAndSendOptions");

  var _observeSign = (0, _classPrivateFieldLooseKey2.default)("observeSign");

  var _observeStatus = (0, _classPrivateFieldLooseKey2.default)("observeStatus");

  var _observeSend = (0, _classPrivateFieldLooseKey2.default)("observeSend");

  var _observeSubscribe = (0, _classPrivateFieldLooseKey2.default)("observeSubscribe");

  var _optionsOrNonce = (0, _classPrivateFieldLooseKey2.default)("optionsOrNonce");

  var _signViaSigner = (0, _classPrivateFieldLooseKey2.default)("signViaSigner");

  var _updateSigner = (0, _classPrivateFieldLooseKey2.default)("updateSigner");

  class Submittable extends ExtrinsicBase {
    constructor(registry, extrinsic) {
      super(registry, extrinsic, {
        version: api.extrinsicType
      });
      Object.defineProperty(this, _ignoreStatusCb, {
        writable: true,
        value: void 0
      });
      Object.defineProperty(this, _transformResult, {
        writable: true,
        value: identity
      });
      Object.defineProperty(this, _makeEraOptions, {
        writable: true,
        value: (options, {
          header,
          mortalLength,
          nonce
        }) => {
          if (!header) {
            if ((0, _util.isNumber)(options.era)) {
              // since we have no header, it is immortal, remove any option overrides
              // so we only supply the genesisHash and no era to the construction
              delete options.era;
              delete options.blockHash;
            }

            return (0, _classPrivateFieldLooseBase2.default)(this, _makeSignOptions)[_makeSignOptions](options, {
              nonce
            });
          }

          return (0, _classPrivateFieldLooseBase2.default)(this, _makeSignOptions)[_makeSignOptions](options, {
            blockHash: header.hash,
            era: this.registry.createType('ExtrinsicEra', {
              current: header.number,
              period: options.era || mortalLength
            }),
            nonce
          });
        }
      });
      Object.defineProperty(this, _makeSignOptions, {
        writable: true,
        value: (options, extras) => {
          return _objectSpread(_objectSpread(_objectSpread({
            blockHash: api.genesisHash,
            genesisHash: api.genesisHash
          }, options), extras), {}, {
            runtimeVersion: api.runtimeVersion,
            signedExtensions: api.registry.signedExtensions,
            version: api.extrinsicType
          });
        }
      });
      Object.defineProperty(this, _makeSignAndSendOptions, {
        writable: true,
        value: (optionsOrStatus, statusCb) => {
          let options = {};

          if ((0, _util.isFunction)(optionsOrStatus)) {
            statusCb = optionsOrStatus;
          } else {
            options = _objectSpread({}, optionsOrStatus);
          }

          return [options, statusCb];
        }
      });
      Object.defineProperty(this, _observeSign, {
        writable: true,
        value: (account, optionsOrNonce) => {
          const address = (0, _util2.isKeyringPair)(account) ? account.address : account.toString();

          const options = (0, _classPrivateFieldLooseBase2.default)(this, _optionsOrNonce)[_optionsOrNonce](optionsOrNonce);

          let updateId;
          return api.derive.tx.signingInfo(address, options.nonce, options.era).pipe((0, _operators.first)(), (0, _operators.mergeMap)(async signingInfo => {
            const eraOptions = (0, _classPrivateFieldLooseBase2.default)(this, _makeEraOptions)[_makeEraOptions](options, signingInfo);

            if ((0, _util2.isKeyringPair)(account)) {
              this.sign(account, eraOptions);
            } else {
              updateId = await (0, _classPrivateFieldLooseBase2.default)(this, _signViaSigner)[_signViaSigner](address, eraOptions, signingInfo.header);
            }
          }), (0, _operators.mapTo)(updateId));
        }
      });
      Object.defineProperty(this, _observeStatus, {
        writable: true,
        value: (hash, status) => {
          if (!status.isFinalized && !status.isInBlock) {
            return (0, _xRxjs.of)((0, _classPrivateFieldLooseBase2.default)(this, _transformResult)[_transformResult](new _Result.SubmittableResult({
              status
            })));
          }

          const blockHash = status.isInBlock ? status.asInBlock : status.asFinalized;
          return api.derive.tx.events(blockHash).pipe((0, _operators.map)(({
            block,
            events
          }) => (0, _classPrivateFieldLooseBase2.default)(this, _transformResult)[_transformResult](new _Result.SubmittableResult({
            events: (0, _util2.filterEvents)(hash, block, events, status),
            status
          }))));
        }
      });
      Object.defineProperty(this, _observeSend, {
        writable: true,
        value: (updateId = -1) => {
          return api.rpc.author.submitExtrinsic(this).pipe((0, _operators.tap)(hash => {
            (0, _classPrivateFieldLooseBase2.default)(this, _updateSigner)[_updateSigner](updateId, hash);
          }));
        }
      });
      Object.defineProperty(this, _observeSubscribe, {
        writable: true,
        value: (updateId = -1) => {
          const hash = this.hash;
          return api.rpc.author.submitAndWatchExtrinsic(this).pipe((0, _operators.switchMap)(status => (0, _classPrivateFieldLooseBase2.default)(this, _observeStatus)[_observeStatus](hash, status)), (0, _operators.tap)(status => {
            (0, _classPrivateFieldLooseBase2.default)(this, _updateSigner)[_updateSigner](updateId, status);
          }));
        }
      });
      Object.defineProperty(this, _optionsOrNonce, {
        writable: true,
        value: (optionsOrNonce = {}) => {
          return (0, _util.isBn)(optionsOrNonce) || (0, _util.isNumber)(optionsOrNonce) ? {
            nonce: optionsOrNonce
          } : optionsOrNonce;
        }
      });
      Object.defineProperty(this, _signViaSigner, {
        writable: true,
        value: async (address, options, header) => {
          const signer = options.signer || api.signer;
          (0, _util.assert)(signer, 'No signer specified, either via api.setSigner or via sign options. You possibly need to pass through an explicit keypair for the origin so it can be used for signing.');
          const payload = this.registry.createType('SignerPayload', _objectSpread(_objectSpread({}, options), {}, {
            address,
            blockNumber: header ? header.number : 0,
            method: this.method
          }));
          let result;

          if (signer.signPayload) {
            result = await signer.signPayload(payload.toPayload());
          } else if (signer.signRaw) {
            result = await signer.signRaw(payload.toRaw());
          } else {
            throw new Error('Invalid signer interface, it should implement either signPayload or signRaw (or both)');
          } // Here we explicitly call `toPayload()` again instead of working with an object
          // (reference) as passed to the signer. This means that we are sure that the
          // payload data is not modified from our inputs, but the signer


          super.addSignature(address, result.signature, payload.toPayload());
          return result.id;
        }
      });
      Object.defineProperty(this, _updateSigner, {
        writable: true,
        value: (updateId, status) => {
          if (updateId !== -1 && api.signer && api.signer.update) {
            api.signer.update(updateId, status);
          }
        }
      });
      (0, _classPrivateFieldLooseBase2.default)(this, _ignoreStatusCb)[_ignoreStatusCb] = apiType === 'rxjs';
    } // dry run an extrinsic


    dryRun(account, optionsOrHash) {
      if ((0, _util.isString)(optionsOrHash) || (0, _util.isU8a)(optionsOrHash)) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-return
        return decorateMethod(() => api.rpc.system.dryRun(this.toHex(), optionsOrHash));
      } // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/no-unsafe-call


      return decorateMethod(() => (0, _classPrivateFieldLooseBase2.default)(this, _observeSign)[_observeSign](account, optionsOrHash).pipe((0, _operators.switchMap)(() => api.rpc.system.dryRun(this.toHex()))))();
    } // calculate the payment info for this transaction (if signed and submitted)


    paymentInfo(account, optionsOrHash) {
      if ((0, _util.isString)(optionsOrHash) || (0, _util.isU8a)(optionsOrHash)) {
        // eslint-disable-next-line @typescript-eslint/no-unsafe-return
        return decorateMethod(() => api.rpc.payment.queryInfo(this.toHex(), optionsOrHash));
      }

      const [allOptions] = (0, _classPrivateFieldLooseBase2.default)(this, _makeSignAndSendOptions)[_makeSignAndSendOptions](optionsOrHash);

      const address = (0, _util2.isKeyringPair)(account) ? account.address : account.toString(); // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/no-unsafe-call

      return decorateMethod(() => api.derive.tx.signingInfo(address, allOptions.nonce, allOptions.era).pipe((0, _operators.first)(), (0, _operators.switchMap)(signingInfo => {
        // setup our options (same way as in signAndSend)
        const eraOptions = (0, _classPrivateFieldLooseBase2.default)(this, _makeEraOptions)[_makeEraOptions](allOptions, signingInfo);

        const signOptions = (0, _classPrivateFieldLooseBase2.default)(this, _makeSignOptions)[_makeSignOptions](eraOptions, {});

        this.signFake(address, signOptions);
        return api.rpc.payment.queryInfo(this.toHex());
      })))();
    } // send with an immediate Hash result


    // send implementation for both immediate Hash and statusCb variants
    send(statusCb) {
      const isSubscription = api.hasSubscriptions && ((0, _classPrivateFieldLooseBase2.default)(this, _ignoreStatusCb)[_ignoreStatusCb] || !!statusCb); // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/no-unsafe-call

      return decorateMethod(isSubscription ? (0, _classPrivateFieldLooseBase2.default)(this, _observeSubscribe)[_observeSubscribe] : (0, _classPrivateFieldLooseBase2.default)(this, _observeSend)[_observeSend])(statusCb);
    }
    /**
     * @description Sign a transaction, returning the this to allow chaining, i.e. .sign(...).send(). When options, e.g. nonce/blockHash are not specified, it will be inferred. To retrieve eg. nonce use `signAsync` (the preferred interface, this is provided for backwards compatibility)
     * @deprecated
     */


    sign(account, optionsOrNonce) {
      super.sign(account, (0, _classPrivateFieldLooseBase2.default)(this, _makeSignOptions)[_makeSignOptions]((0, _classPrivateFieldLooseBase2.default)(this, _optionsOrNonce)[_optionsOrNonce](optionsOrNonce), {}));
      return this;
    }
    /**
     * @description Signs a transaction, returning `this` to allow chaining. E.g.: `sign(...).send()`. Like `.signAndSend` this will retrieve the nonce and blockHash to send the tx with.
     */


    signAsync(account, optionsOrNonce) {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/no-unsafe-call
      return decorateMethod(() => (0, _classPrivateFieldLooseBase2.default)(this, _observeSign)[_observeSign](account, optionsOrNonce).pipe((0, _operators.mapTo)(this)))();
    } // signAndSend with an immediate Hash result


    // signAndSend implementation for all 3 cases above
    signAndSend(account, optionsOrStatus, optionalStatusCb) {
      const [options, statusCb] = (0, _classPrivateFieldLooseBase2.default)(this, _makeSignAndSendOptions)[_makeSignAndSendOptions](optionsOrStatus, optionalStatusCb);

      const isSubscription = api.hasSubscriptions && ((0, _classPrivateFieldLooseBase2.default)(this, _ignoreStatusCb)[_ignoreStatusCb] || !!statusCb); // eslint-disable-next-line @typescript-eslint/no-unsafe-return,@typescript-eslint/no-unsafe-call

      return decorateMethod(() => (0, _classPrivateFieldLooseBase2.default)(this, _observeSign)[_observeSign](account, options).pipe((0, _operators.switchMap)(updateId => isSubscription ? (0, _classPrivateFieldLooseBase2.default)(this, _observeSubscribe)[_observeSubscribe](updateId) : (0, _classPrivateFieldLooseBase2.default)(this, _observeSend)[_observeSend](updateId))) // FIXME This is wrong, SubmittableResult is _not_ a codec
      )(statusCb);
    } // adds a transform to the result, applied before result is returned


    withResultTransform(transform) {
      (0, _classPrivateFieldLooseBase2.default)(this, _transformResult)[_transformResult] = transform;
      return this;
    }

  }

  return Submittable;
}