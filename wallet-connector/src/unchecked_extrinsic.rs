// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generic implementation of an unchecked (pre-verification) extrinsic.

use crate::signed_payload::SignedPayload;
use codec::{Compact, Decode, Encode, EncodeLike, Error, Input};
use frame_support::traits::ExtrinsicCall;
use parity_scale_codec as codec;
use scale_info::{build::Fields, meta_type, Path, StaticTypeInfo, Type, TypeInfo, TypeParameter};
use sp_runtime::{
	generic::CheckedExtrinsic,
	traits::{
		self, Checkable, Extrinsic, ExtrinsicMetadata, MaybeDisplay, Member, SignedExtension,
	},
	transaction_validity::{InvalidTransaction, TransactionValidityError},
	OpaqueExtrinsic,
};
use sp_std::{fmt, prelude::*};

/// Current version of the [`UncheckedExtrinsic`] encoded format.
///
/// This version needs to be bumped if the encoded representation changes.
/// It ensures that if the representation is changed and the format is not known,
/// the decoding fails.
const EXTRINSIC_FORMAT_VERSION: u8 = 4;

/// A extrinsic right from the external world. This is unchecked and so
/// can contain a signature.
#[derive(PartialEq, Eq, Clone)]
pub struct UncheckedExtrinsic<Address, Call, Extra>
where
	Extra: SignedExtension,
{
	/// The signature, address, number of extrinsics have come before from
	/// the same signer and an era describing the longevity of this transaction,
	/// if this is a signed extrinsic.
	pub signature: Option<(Address, Signature, Extra)>,
	/// The function that should be called.
	pub function: Call,
}

impl<Address, Call, Extra> ExtrinsicCall for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: StaticTypeInfo,
	Call: StaticTypeInfo,
	Extra: SignedExtension + StaticTypeInfo,
{
	fn call(&self) -> &Self::Call {
		&self.function
	}
}
use crate::signature::CustomSignature;
use frame_support::dispatch::{DispatchInfo, GetDispatchInfo};
use polkadex_primitives::Signature;
use sp_core::crypto::AccountId32;
use sp_runtime::traits::Verify;

impl<Address, Call, Extra> GetDispatchInfo for UncheckedExtrinsic<Address, Call, Extra>
where
	Call: GetDispatchInfo,
	Extra: SignedExtension,
{
	fn get_dispatch_info(&self) -> DispatchInfo {
		self.function.get_dispatch_info()
	}
}

/// Manual [`TypeInfo`] implementation because of custom encoding. The data is a valid encoded
/// `Vec<u8>`, but requires some logic to extract the signature and payload.
///
/// See [`UncheckedExtrinsic::encode`] and [`UncheckedExtrinsic::decode`].
impl<Address, Call, Extra> TypeInfo for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: StaticTypeInfo,
	Call: StaticTypeInfo,
	Extra: SignedExtension + StaticTypeInfo,
{
	type Identity = UncheckedExtrinsic<Address, Call, Extra>;

	fn type_info() -> Type {
		Type::builder()
			.path(Path::new("UncheckedExtrinsic", module_path!()))
			// Include the type parameter types, even though they are not used directly in any of
			// the described fields. These type definitions can be used by downstream consumers
			// to help construct the custom decoding from the opaque bytes (see below).
			.type_params(vec![
				TypeParameter::new("Address", Some(meta_type::<Address>())),
				TypeParameter::new("Call", Some(meta_type::<Call>())),
				TypeParameter::new("Signature", Some(meta_type::<Signature>())),
				TypeParameter::new("Extra", Some(meta_type::<Extra>())),
			])
			.docs(&["UncheckedExtrinsic raw bytes, requires custom decoding routine"])
			// Because of the custom encoding, we can only accurately describe the encoding as an
			// opaque `Vec<u8>`. Downstream consumers will need to manually implement the codec to
			// encode/decode the `signature` and `function` fields.
			.composite(Fields::unnamed().field(|f| f.ty::<Vec<u8>>()))
	}
}

#[cfg(feature = "std")]
impl<Address, Call, Extra> parity_util_mem::MallocSizeOf
	for UncheckedExtrinsic<Address, Call, Extra>
where
	Extra: SignedExtension,
{
	fn size_of(&self, _ops: &mut parity_util_mem::MallocSizeOfOps) -> usize {
		// Instantiated only in runtime.
		0
	}
}

impl<Address, Call, Extra: SignedExtension> UncheckedExtrinsic<Address, Call, Extra> {
	/// New instance of a signed extrinsic aka "transaction".
	pub fn new_signed(function: Call, signed: Address, signature: Signature, extra: Extra) -> Self {
		Self { signature: Some((signed, signature, extra)), function }
	}

	/// New instance of an unsigned extrinsic aka "inherent".
	pub fn new_unsigned(function: Call) -> Self {
		Self { signature: None, function }
	}
}

impl<Address, Call, Extra: SignedExtension> Extrinsic for UncheckedExtrinsic<Address, Call, Extra> {
	type Call = Call;

	type SignaturePayload = (Address, Signature, Extra);

	fn is_signed(&self) -> Option<bool> {
		Some(self.signature.is_some())
	}

	fn new(function: Call, signed_data: Option<Self::SignaturePayload>) -> Option<Self> {
		Some(if let Some((address, signature, extra)) = signed_data {
			Self::new_signed(function, address, signature, extra)
		} else {
			Self::new_unsigned(function)
		})
	}
}

impl<Address, AccountId, Call, Extra, Lookup> Checkable<Lookup>
	for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: Member + MaybeDisplay,
	Call: Encode + Member,
	Extra: SignedExtension<AccountId = AccountId>,
	AccountId: Member + MaybeDisplay + Into<AccountId32>,
	Lookup: traits::Lookup<Source = Address, Target = AccountId>,
{
	type Checked = CheckedExtrinsic<AccountId, Call, Extra>;

	fn check(self, lookup: &Lookup) -> Result<Self::Checked, TransactionValidityError> {
		Ok(match self.signature {
			Some((signed, signature, extra)) => {
				let signed = lookup.lookup(signed)?;
				let raw_payload = SignedPayload::new(self.function, extra)?;
				let custom_signature = CustomSignature { signature };
				if !raw_payload.using_encoded(|payload| {
					custom_signature.verify(payload, &signed.clone().into())
				}) {
					return Err(InvalidTransaction::BadProof.into())
				}

				let (function, extra, _) = raw_payload.deconstruct();
				CheckedExtrinsic { signed: Some((signed, extra)), function }
			},
			None => CheckedExtrinsic { signed: None, function: self.function },
		})
	}
}

impl<Address, Call, Extra> ExtrinsicMetadata for UncheckedExtrinsic<Address, Call, Extra>
where
	Extra: SignedExtension,
{
	const VERSION: u8 = EXTRINSIC_FORMAT_VERSION;
	type SignedExtensions = Extra;
}

impl<Address, Call, Extra> Decode for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: Decode,
	Call: Decode,
	Extra: SignedExtension,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		// This is a little more complicated than usual since the binary format must be compatible
		// with SCALE's generic `Vec<u8>` type. Basically this just means accepting that there
		// will be a prefix of vector length.
		let expected_length: Compact<u32> = Decode::decode(input)?;
		let before_length = input.remaining_len()?;

		let version = input.read_byte()?;

		let is_signed = version & 0b1000_0000 != 0;
		let version = version & 0b0111_1111;
		if version != EXTRINSIC_FORMAT_VERSION {
			return Err("Invalid transaction version".into())
		}

		let signature = is_signed.then(|| Decode::decode(input)).transpose()?;
		let function = Decode::decode(input)?;

		if let Some((before_length, after_length)) =
			input.remaining_len()?.and_then(|a| before_length.map(|b| (b, a)))
		{
			let length = before_length.saturating_sub(after_length);

			if length != expected_length.0 as usize {
				return Err("Invalid length prefix".into())
			}
		}

		Ok(Self { signature, function })
	}
}

impl<Address, Call, Extra> Encode for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: Encode,
	Call: Encode,
	Extra: SignedExtension,
{
	fn encode(&self) -> Vec<u8> {
		let mut tmp = Vec::with_capacity(sp_std::mem::size_of::<Self>());

		// 1 byte version id.
		match self.signature.as_ref() {
			Some(s) => {
				tmp.push(EXTRINSIC_FORMAT_VERSION | 0b1000_0000);
				s.encode_to(&mut tmp);
			},
			None => {
				tmp.push(EXTRINSIC_FORMAT_VERSION & 0b0111_1111);
			},
		}
		self.function.encode_to(&mut tmp);

		let compact_len = codec::Compact::<u32>(tmp.len() as u32);

		// Allocate the output buffer with the correct length
		let mut output = Vec::with_capacity(compact_len.size_hint() + tmp.len());

		compact_len.encode_to(&mut output);
		output.extend(tmp);

		output
	}
}

impl<Address, Call, Extra> EncodeLike for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: Encode,
	Call: Encode,
	Extra: SignedExtension,
{
}

#[cfg(feature = "std")]
impl<Address: Encode, Call: Encode, Extra: SignedExtension> serde::Serialize
	for UncheckedExtrinsic<Address, Call, Extra>
{
	fn serialize<S>(&self, seq: S) -> Result<S::Ok, S::Error>
	where
		S: ::serde::Serializer,
	{
		self.using_encoded(|bytes| seq.serialize_bytes(bytes))
	}
}

#[cfg(feature = "std")]
impl<'a, Address: Decode, Call: Decode, Extra: SignedExtension> serde::Deserialize<'a>
	for UncheckedExtrinsic<Address, Call, Extra>
{
	fn deserialize<D>(de: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'a>,
	{
		let r = sp_core::bytes::deserialize(de)?;
		Decode::decode(&mut &r[..]).map_err(|e| serde::de::Error::custom(e.to_string()))
	}
}

impl<Address, Call, Extra> fmt::Debug for UncheckedExtrinsic<Address, Call, Extra>
where
	Address: fmt::Debug,
	Call: fmt::Debug,
	Extra: SignedExtension,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"UncheckedExtrinsic({:?}, {:?})",
			self.signature.as_ref().map(|x| (&x.0, &x.2)),
			self.function,
		)
	}
}

impl<Address, Call, Extra> From<UncheckedExtrinsic<Address, Call, Extra>> for OpaqueExtrinsic
where
	Address: Encode,
	Call: Encode,
	Extra: SignedExtension,
{
	fn from(extrinsic: UncheckedExtrinsic<Address, Call, Extra>) -> Self {
		Self::from_bytes(extrinsic.encode().as_slice()).expect(
			"both OpaqueExtrinsic and UncheckedExtrinsic have encoding that is compatible with \
				raw Vec<u8> encoding; qed",
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use parity_scale_codec::{Decode, Encode};
	use sp_runtime::traits::{DispatchInfoOf, SignedExtension};
	type TestAccountId = u64;
	type TestCall = Vec<u8>;

	const TEST_ACCOUNT: TestAccountId = 0;

	// NOTE: this is demonstration. One can simply use `()` for testing.
	#[derive(Debug, Encode, Decode, Clone, Eq, PartialEq, Ord, PartialOrd, TypeInfo)]
	struct TestExtra;
	impl SignedExtension for TestExtra {
		const IDENTIFIER: &'static str = "TestExtra";
		type AccountId = u64;
		type Call = ();
		type AdditionalSigned = ();
		type Pre = ();

		fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
			Ok(())
		}

		fn pre_dispatch(
			self,
			who: &Self::AccountId,
			call: &Self::Call,
			info: &DispatchInfoOf<Self::Call>,
			len: usize,
		) -> Result<Self::Pre, TransactionValidityError> {
			self.validate(who, call, info, len).map(|_| ())
		}
	}

	type Ex = UncheckedExtrinsic<TestAccountId, TestCall, TestExtra>;

	#[test]
	fn unsigned_codec_should_work() {
		let ux = Ex::new_unsigned(vec![0u8; 0]);
		let encoded = ux.encode();
		assert_eq!(Ex::decode(&mut &encoded[..]), Ok(ux));
	}

	#[test]
	fn invalid_length_prefix_is_detected() {
		let ux = Ex::new_unsigned(vec![0u8; 0]);
		let mut encoded = ux.encode();

		let length = Compact::<u32>::decode(&mut &encoded[..]).unwrap();
		Compact(length.0 + 10).encode_to(&mut &mut encoded[..1]);

		assert_eq!(Ex::decode(&mut &encoded[..]), Err("Invalid length prefix".into()));
	}

	#[test]
	fn signed_codec_should_work() {
		let ux = Ex::new_signed(
			vec![0u8; 0],
			TEST_ACCOUNT,
			Signature::Sr25519(sp_core::sr25519::Signature([0; 64])),
			TestExtra,
		);
		let encoded = ux.encode();
		assert_eq!(Ex::decode(&mut &encoded[..]), Ok(ux));
	}

	#[test]
	fn large_signed_codec_should_work() {
		let ux = Ex::new_signed(
			vec![0u8; 0],
			TEST_ACCOUNT,
			Signature::Sr25519(sp_core::sr25519::Signature([0; 64])),
			TestExtra,
		);
		let encoded = ux.encode();
		assert_eq!(Ex::decode(&mut &encoded[..]), Ok(ux));
	}

	#[test]
	fn encoding_matches_vec() {
		let ex = Ex::new_unsigned(vec![0u8; 0]);
		let encoded = ex.encode();
		let decoded = Ex::decode(&mut encoded.as_slice()).unwrap();
		assert_eq!(decoded, ex);
		let as_vec: Vec<u8> = Decode::decode(&mut encoded.as_slice()).unwrap();
		assert_eq!(as_vec.encode(), encoded);
	}

	#[test]
	fn conversion_to_opaque() {
		let ux = Ex::new_unsigned(vec![0u8; 0]);
		let encoded = ux.encode();
		let opaque: OpaqueExtrinsic = ux.into();
		let opaque_encoded = opaque.encode();
		assert_eq!(opaque_encoded, encoded);
	}

	#[test]
	fn large_bad_prefix_should_work() {
		let encoded = Compact::<u32>::from(u32::MAX).encode();
		assert_eq!(
			Ex::decode(&mut &encoded[..]),
			Err(Error::from("Not enough data to fill buffer"))
		);
	}
}
