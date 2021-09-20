use fixed::{types::extra::U32, FixedU128};
use ink_primitives::Key;
use ink_primitives::KeyPtr;
#[cfg(feature = "std")]
use ink_storage::traits::StorageLayout;
use ink_storage::traits::{forward_clear_packed, forward_pull_packed, forward_push_packed};
use ink_storage::traits::{PackedLayout, SpreadLayout};
use primitive_types::H160;
use scale_info::{build::Fields, Path, Type, TypeInfo};

pub type ExchangeRate = FixedU128<U32>;
pub type Ratio = FixedU128<U32>;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct TokenAddress(H160);

impl PackedLayout for TokenAddress {
    #[inline]
    fn pull_packed(&mut self, _at: &Key) {}
    #[inline]
    fn push_packed(&self, _at: &Key) {}
    #[inline]
    fn clear_packed(&self, _at: &Key) {}
}

impl SpreadLayout for TokenAddress {
    const FOOTPRINT: u64 = 1;
    const REQUIRES_DEEP_CLEAN_UP: bool = false;

    #[inline]
    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        forward_pull_packed::<Self>(ptr)
    }

    #[inline]
    fn push_spread(&self, ptr: &mut KeyPtr) {
        forward_push_packed::<Self>(self, ptr)
    }

    #[inline]
    fn clear_spread(&self, ptr: &mut KeyPtr) {
        forward_clear_packed::<Self>(self, ptr)
    }
}

impl scale::Encode for TokenAddress {
    #[inline(always)]
    fn size_hint(&self) -> usize {
        20
    }

    #[inline]
    fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
        dest.write(&self.0.as_bytes())
    }
}

impl scale::Decode for TokenAddress {
    #[inline]
    fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
        // As H160 is just a glorified wrapper around 20 bytes we simply
        // forward this information directly in the TypeInfo trait.
        // There is no need for more elaborate type information.
        let dec = <[u8; 20] as scale::Decode>::decode(input)?;
        Ok(Self(H160(dec)))
    }
}

impl TypeInfo for TokenAddress {
    type Identity = Self;

    fn type_info() -> Type {
        Type::builder()
            .path(Path::new("TokenAddress", "pool"))
            .composite(Fields::unnamed().field_of::<[u8; 20]>("[u8; 20]"))
    }
}

impl TokenAddress {
    /// Returns a shared reference to the wrapped H160 instance.
    ///
    /// # Note
    ///
    /// This is an inherent method to avoid name resolution problems.
    #[inline]
    pub fn get(self: &Self) -> &H160 {
        &self.0
    }

    /// Returns an exclusive reference to the wrapped H160 instance.
    ///
    /// # Note
    ///
    /// This is an inherent method to avoid name resolution problems.
    #[inline]
    pub fn get_mut(self: &mut Self) -> &mut H160 {
        &mut self.0
    }

    /// Consumes the wrapper to return the owned H160 instance.
    ///
    /// # Note
    ///
    /// This is an inherent method to avoid name resolution problems.
    #[inline]
    pub fn into_inner(self: Self) -> H160 {
        self.0
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        TokenAddress(H160::from_slice(slice))
    }
}

impl core::ops::Deref for TokenAddress {
    type Target = H160;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl core::ops::DerefMut for TokenAddress {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    scale::Encode,
    scale::Decode,
    PartialOrd,
    Ord,
    PackedLayout,
    SpreadLayout,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct TradingPair(TokenAddress, TokenAddress);

impl TradingPair {
    pub fn from_currency_ids(
        currency_id_a: TokenAddress,
        currency_id_b: TokenAddress,
    ) -> Option<Self> {
        if currency_id_a != currency_id_b {
            if currency_id_a > currency_id_b {
                Some(TradingPair(currency_id_b, currency_id_a))
            } else {
                Some(TradingPair(currency_id_a, currency_id_b))
            }
        } else {
            None
        }
    }

    pub fn first(&self) -> TokenAddress {
        self.0
    }

    pub fn second(&self) -> TokenAddress {
        self.1
    }
}
