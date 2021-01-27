"use strict";

var _interopRequireWildcard = require("@babel/runtime/helpers/interopRequireWildcard");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isReady = isReady;
exports.waitReady = waitReady;
exports.twox = exports.sha512 = exports.scrypt = exports.pbkdf2 = exports.keccak256 = exports.blake2b = exports.vrfVerify = exports.vrfSign = exports.sr25519Verify = exports.sr25519Sign = exports.sr25519KeypairFromSeed = exports.sr25519DerivePublicSoft = exports.sr25519DeriveKeypairSoft = exports.sr25519DeriveKeypairHard = exports.ed25519Verify = exports.ed25519Sign = exports.ed25519KeypairFromSeed = exports.bip39Validate = exports.bip39ToSeed = exports.bip39ToMiniSecret = exports.bip39ToEntropy = exports.bip39Generate = void 0;

require("./detectPackage");

var _wasmCryptoAsmjs = require("@polkadot/wasm-crypto-asmjs");

var _wasmCryptoWasm = require("@polkadot/wasm-crypto-wasm");

var _bridge = require("./bridge");

var imports = _interopRequireWildcard(require("./imports"));

// Copyright 2019-2021 @polkadot/wasm-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const wasmPromise = (0, _bridge.initWasm)(_wasmCryptoWasm.wasmBytes, _wasmCryptoAsmjs.asmJsInit, imports).catch(() => null);
const bip39Generate = (0, _bridge.withWasm)(wasm => words => {
  wasm.ext_bip39_generate(8, words);
  return (0, _bridge.resultString)();
});
exports.bip39Generate = bip39Generate;
const bip39ToEntropy = (0, _bridge.withWasm)(wasm => phrase => {
  const [ptr0, len0] = (0, _bridge.allocString)(phrase);
  wasm.ext_bip39_to_entropy(8, ptr0, len0);
  return (0, _bridge.resultU8a)();
});
exports.bip39ToEntropy = bip39ToEntropy;
const bip39ToMiniSecret = (0, _bridge.withWasm)(wasm => (phrase, password) => {
  const [ptr0, len0] = (0, _bridge.allocString)(phrase);
  const [ptr1, len1] = (0, _bridge.allocString)(password);
  wasm.ext_bip39_to_mini_secret(8, ptr0, len0, ptr1, len1);
  return (0, _bridge.resultU8a)();
});
exports.bip39ToMiniSecret = bip39ToMiniSecret;
const bip39ToSeed = (0, _bridge.withWasm)(wasm => (phrase, password) => {
  const [ptr0, len0] = (0, _bridge.allocString)(phrase);
  const [ptr1, len1] = (0, _bridge.allocString)(password);
  wasm.ext_bip39_to_seed(8, ptr0, len0, ptr1, len1);
  return (0, _bridge.resultU8a)();
});
exports.bip39ToSeed = bip39ToSeed;
const bip39Validate = (0, _bridge.withWasm)(wasm => phrase => {
  const [ptr0, len0] = (0, _bridge.allocString)(phrase);
  const ret = wasm.ext_bip39_validate(ptr0, len0);
  return ret !== 0;
});
exports.bip39Validate = bip39Validate;
const ed25519KeypairFromSeed = (0, _bridge.withWasm)(wasm => seed => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(seed);
  wasm.ext_ed_from_seed(8, ptr0, len0);
  return (0, _bridge.resultU8a)();
});
exports.ed25519KeypairFromSeed = ed25519KeypairFromSeed;
const ed25519Sign = (0, _bridge.withWasm)(wasm => (pubkey, seckey, message) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pubkey);
  const [ptr1, len1] = (0, _bridge.allocU8a)(seckey);
  const [ptr2, len2] = (0, _bridge.allocU8a)(message);
  wasm.ext_ed_sign(8, ptr0, len0, ptr1, len1, ptr2, len2);
  return (0, _bridge.resultU8a)();
});
exports.ed25519Sign = ed25519Sign;
const ed25519Verify = (0, _bridge.withWasm)(wasm => (signature, message, pubkey) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(signature);
  const [ptr1, len1] = (0, _bridge.allocU8a)(message);
  const [ptr2, len2] = (0, _bridge.allocU8a)(pubkey);
  const ret = wasm.ext_ed_verify(ptr0, len0, ptr1, len1, ptr2, len2);
  return ret !== 0;
});
exports.ed25519Verify = ed25519Verify;
const sr25519DeriveKeypairHard = (0, _bridge.withWasm)(wasm => (pair, cc) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pair);
  const [ptr1, len1] = (0, _bridge.allocU8a)(cc);
  wasm.ext_sr_derive_keypair_hard(8, ptr0, len0, ptr1, len1);
  return (0, _bridge.resultU8a)();
});
exports.sr25519DeriveKeypairHard = sr25519DeriveKeypairHard;
const sr25519DeriveKeypairSoft = (0, _bridge.withWasm)(wasm => (pair, cc) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pair);
  const [ptr1, len1] = (0, _bridge.allocU8a)(cc);
  wasm.ext_sr_derive_keypair_soft(8, ptr0, len0, ptr1, len1);
  return (0, _bridge.resultU8a)();
});
exports.sr25519DeriveKeypairSoft = sr25519DeriveKeypairSoft;
const sr25519DerivePublicSoft = (0, _bridge.withWasm)(wasm => (pubkey, cc) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pubkey);
  const [ptr1, len1] = (0, _bridge.allocU8a)(cc);
  wasm.ext_sr_derive_public_soft(8, ptr0, len0, ptr1, len1);
  return (0, _bridge.resultU8a)();
});
exports.sr25519DerivePublicSoft = sr25519DerivePublicSoft;
const sr25519KeypairFromSeed = (0, _bridge.withWasm)(wasm => seed => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(seed);
  wasm.ext_sr_from_seed(8, ptr0, len0);
  return (0, _bridge.resultU8a)();
});
exports.sr25519KeypairFromSeed = sr25519KeypairFromSeed;
const sr25519Sign = (0, _bridge.withWasm)(wasm => (pubkey, secret, message) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pubkey);
  const [ptr1, len1] = (0, _bridge.allocU8a)(secret);
  const [ptr2, len2] = (0, _bridge.allocU8a)(message);
  wasm.ext_sr_sign(8, ptr0, len0, ptr1, len1, ptr2, len2);
  return (0, _bridge.resultU8a)();
});
exports.sr25519Sign = sr25519Sign;
const sr25519Verify = (0, _bridge.withWasm)(wasm => (signature, message, pubkey) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(signature);
  const [ptr1, len1] = (0, _bridge.allocU8a)(message);
  const [ptr2, len2] = (0, _bridge.allocU8a)(pubkey);
  const ret = wasm.ext_sr_verify(ptr0, len0, ptr1, len1, ptr2, len2);
  return ret !== 0;
});
exports.sr25519Verify = sr25519Verify;
const vrfSign = (0, _bridge.withWasm)(wasm => (secret, context, message, extra) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(secret);
  const [ptr1, len1] = (0, _bridge.allocU8a)(context);
  const [ptr2, len2] = (0, _bridge.allocU8a)(message);
  const [ptr3, len3] = (0, _bridge.allocU8a)(extra);
  wasm.ext_vrf_sign(8, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
  return (0, _bridge.resultU8a)();
});
exports.vrfSign = vrfSign;
const vrfVerify = (0, _bridge.withWasm)(wasm => (pubkey, context, message, extra, outAndProof) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(pubkey);
  const [ptr1, len1] = (0, _bridge.allocU8a)(context);
  const [ptr2, len2] = (0, _bridge.allocU8a)(message);
  const [ptr3, len3] = (0, _bridge.allocU8a)(extra);
  const [ptr4, len4] = (0, _bridge.allocU8a)(outAndProof);
  const ret = wasm.ext_vrf_verify(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3, ptr4, len4);
  return ret !== 0;
});
exports.vrfVerify = vrfVerify;
const blake2b = (0, _bridge.withWasm)(wasm => (data, key, size) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(data);
  const [ptr1, len1] = (0, _bridge.allocU8a)(key);
  wasm.ext_blake2b(8, ptr0, len0, ptr1, len1, size);
  return (0, _bridge.resultU8a)();
});
exports.blake2b = blake2b;
const keccak256 = (0, _bridge.withWasm)(wasm => data => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(data);
  wasm.ext_keccak256(8, ptr0, len0);
  return (0, _bridge.resultU8a)();
});
exports.keccak256 = keccak256;
const pbkdf2 = (0, _bridge.withWasm)(wasm => (data, salt, rounds) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(data);
  const [ptr1, len1] = (0, _bridge.allocU8a)(salt);
  wasm.ext_pbkdf2(8, ptr0, len0, ptr1, len1, rounds);
  return (0, _bridge.resultU8a)();
});
exports.pbkdf2 = pbkdf2;
const scrypt = (0, _bridge.withWasm)(wasm => (password, salt, log2n, r, p) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(password);
  const [ptr1, len1] = (0, _bridge.allocU8a)(salt);
  wasm.ext_scrypt(8, ptr0, len0, ptr1, len1, log2n, r, p);
  return (0, _bridge.resultU8a)();
});
exports.scrypt = scrypt;
const sha512 = (0, _bridge.withWasm)(wasm => data => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(data);
  wasm.ext_sha512(8, ptr0, len0);
  return (0, _bridge.resultU8a)();
});
exports.sha512 = sha512;
const twox = (0, _bridge.withWasm)(wasm => (data, rounds) => {
  const [ptr0, len0] = (0, _bridge.allocU8a)(data);
  wasm.ext_twox(8, ptr0, len0, rounds);
  return (0, _bridge.resultU8a)();
});
exports.twox = twox;

function isReady() {
  return !!(0, _bridge.getWasm)();
}

function waitReady() {
  return wasmPromise.then(() => isReady());
}