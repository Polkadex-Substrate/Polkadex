use crate::ethereum::ethereum_signing;
use parity_scale_codec::{Encode, EncodeLike};
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::SignedExtension, transaction_validity::TransactionValidityError};
use sp_std::vec::Vec;

/// A payload that has been signed for an unchecked extrinsics.
///
/// Note that the payload that we sign to produce unchecked extrinsic signature
/// is going to be different than the `SignaturePayload` - so the thing the extrinsic
/// actually contains.
pub struct SignedPayload<Call, Extra: SignedExtension>((Call, Extra, Extra::AdditionalSigned));

impl<Call, Extra> SignedPayload<Call, Extra>
where
	Call: Encode + Clone,
	Extra: SignedExtension + Clone,
{
	/// Create new `SignedPayload`.
	///
	/// This function may fail if `additional_signed` of `Extra` is not available.
	pub fn new(call: Call, extra: Extra) -> Result<Self, TransactionValidityError> {
		let additional_signed = extra.additional_signed()?;
		let raw_payload = (call, extra, additional_signed);
		Ok(Self(raw_payload))
	}

	/// Create new `SignedPayload` from raw components.
	pub fn from_raw(call: Call, extra: Extra, additional_signed: Extra::AdditionalSigned) -> Self {
		Self((call, extra, additional_signed))
	}

	/// Deconstruct the payload into it's components.
	pub fn deconstruct(self) -> (Call, Extra, Extra::AdditionalSigned) {
		self.0
	}
}

impl<Call, Extra> Encode for SignedPayload<Call, Extra>
where
	Call: Encode + Clone,
	Extra: SignedExtension + Clone,
{
	/// Get an encoded version of this payload.
	///
	/// Payloads longer than 256 bytes are going to be `blake2_256`-hashed.
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		let (_, _, additional_signed) = &self.0;
		let signature_scheme = parse_signature_scheme::<Extra>(additional_signed);
		self.0.using_encoded(|payload| {
			match signature_scheme {
				// Ethereum like signing
				1 => f(&ethereum_signing(payload)),
				// Substrate Generic Signing
				_ => f(&substrate_signing(payload)[..]),
			}
		})
	}
}

impl<Call, Extra> EncodeLike for SignedPayload<Call, Extra>
where
	Call: Encode + Clone,
	Extra: SignedExtension + Clone,
{
}

pub fn substrate_signing(payload: &[u8]) -> Vec<u8> {
	if payload.len() > 256 {
		blake2_256(payload).to_vec()
	} else {
		payload.to_vec()
	}
}

pub fn parse_signature_scheme<Extra: SignedExtension>(
	additional_signed: &Extra::AdditionalSigned,
) -> u8 {
	let encoded = additional_signed.encode();
	if encoded.is_empty() {
		return 0
	}
	encoded[encoded.len() - 1]
}
