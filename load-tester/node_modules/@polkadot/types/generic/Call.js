"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.GenericCall = exports.GenericCallIndex = void 0;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _util = require("@polkadot/util");

var _Struct = require("../codec/Struct");

var _U8aFixed = require("../codec/U8aFixed");

var _createClass = require("../create/createClass");

var _getTypeDef = require("../create/getTypeDef");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

/**
 * Get a mapping of `argument name -> argument type` for the function, from
 * its metadata.
 *
 * @param meta - The function metadata used to get the definition.
 * @internal
 */
function getArgsDef(registry, meta) {
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  return GenericCall.filterOrigin(meta).reduce((result, {
    name,
    type
  }) => {
    const Type = (0, _createClass.getTypeClass)(registry, (0, _getTypeDef.getTypeDef)(type));
    result[name.toString()] = Type;
    return result;
  }, {});
}
/** @internal */


function decodeCallViaObject(registry, value, _meta) {
  // we only pass args/methodsIndex out
  const {
    args,
    callIndex
  } = value; // Get the correct lookupIndex
  // eslint-disable-next-line @typescript-eslint/no-use-before-define

  const lookupIndex = callIndex instanceof GenericCallIndex ? callIndex.toU8a() : callIndex; // Find metadata with callIndex

  const meta = _meta || registry.findMetaCall(lookupIndex).meta;

  return {
    args,
    argsDef: getArgsDef(registry, meta),
    callIndex,
    meta
  };
}
/** @internal */


function decodeCallViaU8a(registry, value, _meta) {
  // We need 2 bytes for the callIndex
  const callIndex = new Uint8Array(2);
  callIndex.set(value.subarray(0, 2), 0); // Find metadata with callIndex

  const meta = _meta || registry.findMetaCall(callIndex).meta;

  return {
    args: value.subarray(2),
    argsDef: getArgsDef(registry, meta),
    callIndex,
    meta
  };
}
/**
 * Decode input to pass into constructor.
 *
 * @param value - Value to decode, one of:
 * - hex
 * - Uint8Array
 * - {@see DecodeMethodInput}
 * @param _meta - Metadata to use, so that `injectMethods` lookup is not
 * necessary.
 * @internal
 */


function decodeCall(registry, value = new Uint8Array(), _meta) {
  if ((0, _util.isHex)(value) || (0, _util.isU8a)(value)) {
    return decodeCallViaU8a(registry, (0, _util.u8aToU8a)(value), _meta);
  } else if ((0, _util.isObject)(value) && value.callIndex && value.args) {
    return decodeCallViaObject(registry, value, _meta);
  }

  throw new Error(`Call: Cannot decode value '${value}' of type ${typeof value}`);
}
/**
 * @name GenericCallIndex
 * @description
 * A wrapper around the `[sectionIndex, methodIndex]` value that uniquely identifies a method
 */


class GenericCallIndex extends _U8aFixed.U8aFixed {
  constructor(registry, value) {
    super(registry, value, 16);
  }

}
/**
 * @name GenericCall
 * @description
 * Extrinsic function descriptor
 */


exports.GenericCallIndex = GenericCallIndex;

class GenericCall extends _Struct.Struct {
  constructor(registry, value, meta) {
    const decoded = decodeCall(registry, value, meta);

    try {
      super(registry, {
        callIndex: GenericCallIndex,
        // eslint-disable-next-line sort-keys
        args: _Struct.Struct.with(decoded.argsDef)
      }, decoded);
      this._meta = void 0;
    } catch (error) {
      let method = 'unknown.unknown';

      try {
        const c = registry.findMetaCall(decoded.callIndex);
        method = `${c.section}.${c.method}`;
      } catch (error) {// ignore
      }

      throw new Error(`Call: failed decoding ${method}:: ${error.message}`);
    }

    this._meta = decoded.meta;
  } // If the extrinsic function has an argument of type `Origin`, we ignore it


  static filterOrigin(meta) {
    // FIXME should be `arg.type !== Origin`, but doesn't work...
    return meta ? meta.args.filter(({
      type
    }) => type.toString() !== 'Origin') : [];
  }
  /**
   * @description The arguments for the function call
   */


  get args() {
    // FIXME This should return a Struct instead of an Array
    return [...this.get('args').values()];
  }
  /**
   * @description The argument definitions
   */


  get argsDef() {
    return getArgsDef(this.registry, this.meta);
  }
  /**
   * @description The encoded `[sectionIndex, methodIndex]` identifier
   */


  get callIndex() {
    return this.get('callIndex').toU8a();
  }
  /**
   * @description The encoded data
   */


  get data() {
    return this.get('args').toU8a();
  }
  /**
   * @description The [[FunctionMetadata]]
   */


  get meta() {
    return this._meta;
  }
  /**
   * @description Returns the name of the method
   */


  get method() {
    return this.registry.findMetaCall(this.callIndex).method;
  }
  /**
   * @description Returns the module containing the method
   */


  get section() {
    return this.registry.findMetaCall(this.callIndex).section;
  }
  /**
   * @description Checks if the source matches this in type
   */


  is(other) {
    return other.callIndex[0] === this.callIndex[0] && other.callIndex[1] === this.callIndex[1];
  }
  /**
   * @description Converts the Object to to a human-friendly JSON, with additional fields, expansion and formatting of information
   */


  toHuman(isExpanded) {
    var _call, _call2;

    let call;

    try {
      call = this.registry.findMetaCall(this.callIndex);
    } catch (error) {// swallow
    }

    return _objectSpread({
      args: this.args.map(arg => arg.toHuman(isExpanded)),
      // args: this.args.map((arg, index) => call
      //   ? { [call.meta.args[index].name.toString()]: arg.toHuman(isExpanded) }
      //   : arg.toHuman(isExpanded)
      // ),
      // callIndex: u8aToHex(this.callIndex),
      method: (_call = call) === null || _call === void 0 ? void 0 : _call.method,
      section: (_call2 = call) === null || _call2 === void 0 ? void 0 : _call2.section
    }, isExpanded && call ? {
      documentation: call.meta.documentation.map(d => d.toString())
    } : {});
  }
  /**
   * @description Returns the base runtime type name for this instance
   */


  toRawType() {
    return 'Call';
  }

}

exports.GenericCall = GenericCall;