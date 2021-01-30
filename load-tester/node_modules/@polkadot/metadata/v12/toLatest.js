"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.toLatest = toLatest;

var _defineProperty2 = _interopRequireDefault(require("@babel/runtime/helpers/defineProperty"));

var _typesKnown = require("@polkadot/types-known");

var _util = require("@polkadot/util");

function ownKeys(object, enumerableOnly) { var keys = Object.keys(object); if (Object.getOwnPropertySymbols) { var symbols = Object.getOwnPropertySymbols(object); if (enumerableOnly) symbols = symbols.filter(function (sym) { return Object.getOwnPropertyDescriptor(object, sym).enumerable; }); keys.push.apply(keys, symbols); } return keys; }

function _objectSpread(target) { for (var i = 1; i < arguments.length; i++) { var source = arguments[i] != null ? arguments[i] : {}; if (i % 2) { ownKeys(Object(source), true).forEach(function (key) { (0, _defineProperty2.default)(target, key, source[key]); }); } else if (Object.getOwnPropertyDescriptors) { Object.defineProperties(target, Object.getOwnPropertyDescriptors(source)); } else { ownKeys(Object(source)).forEach(function (key) { Object.defineProperty(target, key, Object.getOwnPropertyDescriptor(source, key)); }); } } return target; }

// Since we don't have insight into the origin specification, we can only define what we know about
// in a pure Substrate/Polkadot implementation, any other custom origins won't be handled at all
const KNOWN_ORIGINS = {
  Council: 'CollectiveOrigin',
  System: 'SystemOrigin',
  TechnicalCommittee: 'CollectiveOrigin'
};
/**
 * Find and apply the correct type override
 * @internal
 **/

function setTypeOverride(sectionTypes, type) {
  const override = Object.keys(sectionTypes).find(aliased => type.eq(aliased));

  if (override) {
    type.setOverride(sectionTypes[override]);
  } else {
    // FIXME: NOT happy with this approach, but gets over the initial hump cased by (Vec<Announcement>,BalanceOf)
    const orig = type.toString();
    const alias = Object.entries(sectionTypes).reduce((result, [from, to]) => [['<', '>'], ['<', ','], [',', '>'], ['(', ')'], ['(', ','], [',', ','], [',', ')']].reduce((result, [one, two]) => result.replace(`${one}${from}${two}`, `${one}${to}${two}`), result), orig);

    if (orig !== alias) {
      type.setOverride(alias);
    }
  }
}
/**
 * Apply module-specific type overrides (always be done as part of toLatest)
 * @internal
 **/


function convertCalls(registry, calls, sectionTypes) {
  return calls.map(c => {
    c.args.forEach(({
      type
    }) => setTypeOverride(sectionTypes, type));
    return registry.createType('FunctionMetadataLatest', c);
  });
}
/**
 * Apply module-specific type overrides (always be done as part of toLatest)
 * @internal
 */


function convertConstants(registry, constants, sectionTypes) {
  return constants.map(c => {
    setTypeOverride(sectionTypes, c.type);
    return registry.createType('ModuleConstantMetadataLatest', c);
  });
}
/**
 * Apply module-specific type overrides (always be done as part of toLatest)
 * @internal
 **/


function convertEvents(registry, events, sectionTypes) {
  return events.map(e => {
    e.args.forEach(type => setTypeOverride(sectionTypes, type));
    return registry.createType('EventMetadataLatest', e);
  });
}
/**
 * Apply module-specific storage type overrides (always part of toLatest)
 * @internal
 **/


function convertStorage(registry, {
  items,
  prefix
}, sectionTypes) {
  return registry.createType('StorageMetadataLatest', {
    items: items.map(s => {
      let resultType;

      if (s.type.isMap) {
        resultType = s.type.asMap.value;
      } else if (s.type.isDoubleMap) {
        resultType = s.type.asDoubleMap.value;
      } else {
        resultType = s.type.asPlain;
      }

      setTypeOverride(sectionTypes, resultType);
      return registry.createType('StorageEntryMetadataLatest', s);
    }),
    prefix
  });
} // generate & register the OriginCaller type


function registerOriginCaller(registry, modules, metaVersion) {
  registry.register({
    OriginCaller: {
      _enum: modules.map((mod, index) => [mod.name.toString(), metaVersion >= 12 ? mod.index.toNumber() : index]).sort((a, b) => a[1] - b[1]).reduce((result, [name, index]) => {
        for (let i = Object.keys(result).length; i < index; i++) {
          result[`Empty${i}`] = 'Null';
        }

        result[name] = KNOWN_ORIGINS[name] || 'Null';
        return result;
      }, {})
    }
  });
}
/** @internal */


function createModule(registry, mod, {
  calls,
  constants,
  events,
  storage
}) {
  const sectionTypes = (0, _typesKnown.getModuleTypes)(registry, (0, _util.stringCamelCase)(mod.name));
  return registry.createType('ModuleMetadataLatest', _objectSpread(_objectSpread({}, mod), {}, {
    calls: calls && convertCalls(registry, calls, sectionTypes),
    constants: convertConstants(registry, constants, sectionTypes),
    events: events && convertEvents(registry, events, sectionTypes),
    storage: storage && convertStorage(registry, storage, sectionTypes)
  }));
}
/**
 * Convert the Metadata (which is an alias) to latest - effectively this _always_ get applied to the top-level &
 * most-recent metadata, since it allows us a chance to actually apply call and storage specific type aliasses
 * @internal
 **/


function toLatest(registry, {
  extrinsic,
  modules
}, metaVersion) {
  registerOriginCaller(registry, modules, metaVersion);
  return registry.createType('MetadataLatest', {
    extrinsic,
    modules: modules.map(mod => createModule(registry, mod, {
      calls: mod.calls.unwrapOr(null),
      constants: mod.constants,
      events: mod.events.unwrapOr(null),
      storage: mod.storage.unwrapOr(null)
    }))
  });
}