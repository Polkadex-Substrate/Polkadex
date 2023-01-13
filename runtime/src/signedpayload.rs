use parity_scale_codec::{Encode, EncodeLike};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::SignedExtension;
use sp_runtime::transaction_validity::TransactionValidityError;

/// A payload that has been signed for an unchecked extrinsics.
///
/// Note that the payload that we sign to produce unchecked extrinsic signature
/// is going to be different than the `SignaturePayload` - so the thing the extrinsic
/// actually contains.
pub struct SignedPayload<Call, Extra: SignedExtension>(SignedPayloadInner<Call,Extra>);


#[derive(Encode, Clone, Copy)]
pub struct SignedPayloadInner<Call, Extra: SignedExtension> {
    call: Call,
    extra: Extra,
    additional_signed: Extra::AdditionalSigned
}

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
        let raw_payload = Self(SignedPayloadInner{call, extra, additional_signed});
        Ok(raw_payload)
    }

    /// Create new `SignedPayload` from raw components.
    pub fn from_raw(call: Call, extra: Extra, additional_signed: Extra::AdditionalSigned) -> Self {
        Self(SignedPayloadInner{call, extra, additional_signed})
    }

    /// Deconstruct the payload into it's components.
    pub fn deconstruct(self) -> (Call, Extra, Extra::AdditionalSigned) {
        (self.0.call,self.0.extra,self.0.additional_signed)
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
        self.0.using_encoded(|payload| {
            if payload.len() > 256 {
                f(&blake2_256(payload)[..])
            } else {
                f(payload)
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