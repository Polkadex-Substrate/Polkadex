// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::Balance;
#[cfg(not(feature = "std"))]
use codec::alloc::string::ToString;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	ensure,
	traits::{
		tokens::{Fortitude, Precision, Preservation},
		Get,
	},
};
use pallet_asset_conversion::{MultiAssetIdConversionResult, MultiAssetIdConverter};
#[cfg(not(feature = "std"))]
use scale_info::prelude::{format, string::String};
use scale_info::TypeInfo;
use serde::{
	de::{Error, MapAccess, Unexpected, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
use sp_core::RuntimeDebug;
use sp_runtime::{DispatchError, SaturatedConversion};
use sp_std::fmt::{Display, Formatter};

/// Resolver trait for handling different types of assets for deposit and withdrawal operations
pub trait Resolver<
	AccountId,
	Native: frame_support::traits::tokens::fungible::Mutate<AccountId>
		+ frame_support::traits::tokens::fungible::Inspect<AccountId>,
	Others: frame_support::traits::tokens::fungibles::Mutate<AccountId>
		+ frame_support::traits::tokens::fungibles::Inspect<AccountId>
		+ frame_support::traits::tokens::fungibles::Create<AccountId>,
	AssetId: Into<Others::AssetId> + sp_std::cmp::PartialEq + Copy,
	NativeAssetId: Get<AssetId>,
>
{
	/// Deposit will mint new tokens if asset is non native and in case of native, will transfer
	/// native tokens from `NativeLockingAccount` to `who`
	fn resolver_deposit(
		asset: AssetId,
		amount: Balance,
		who: &AccountId,
		admin: AccountId,
		min_balance: Balance,
		locking_account: AccountId,
	) -> Result<(), DispatchError> {
		if asset == NativeAssetId::get() {
			Native::transfer(
				&locking_account,
				who,
				amount.saturated_into(),
				Preservation::Preserve,
			)?;
		} else {
			if !Others::asset_exists(asset.into()) {
				Others::create(asset.into(), admin, true, min_balance.saturated_into())?;
			}
			Others::mint_into(asset.into(), who, amount.saturated_into())?;
		}
		Ok(())
	}

	/// Deposit will burn tokens if asset is non native and in case of native, will transfer
	/// native tokens from `who` to `NativeLockingAccount`
	fn resolver_withdraw(
		asset: AssetId,
		amount: Balance,
		who: &AccountId,
		locking_account: AccountId,
	) -> Result<(), DispatchError> {
		if asset == NativeAssetId::get() {
			Native::transfer(
				who,
				&locking_account,
				amount.saturated_into(),
				Preservation::Preserve,
			)?;
		} else {
			Others::burn_from(
				asset.into(),
				who,
				amount.saturated_into(),
				Precision::Exact,
				Fortitude::Polite,
			)?;
		}
		Ok(())
	}

	/// Create New Asset
	fn resolve_create(
		asset: AssetId,
		admin: AccountId,
		min_balance: Balance,
	) -> Result<(), DispatchError> {
		ensure!(asset != NativeAssetId::get(), DispatchError::Other("Cannot create Native Asset"));
		ensure!(!Others::asset_exists(asset.into()), DispatchError::Other("Asset already exists"));
		Others::create(asset.into(), admin, true, min_balance.saturated_into())?;
		Ok(())
	}

	///Transfer Asset
	fn resolve_transfer(
		asset: AssetId,
		from: &AccountId,
		to: &AccountId,
		amount: Balance,
	) -> Result<(), DispatchError> {
		if asset == NativeAssetId::get() {
			Native::transfer(from, to, amount.saturated_into(), Preservation::Preserve)?;
		} else {
			Others::transfer(
				asset.into(),
				from,
				to,
				amount.saturated_into(),
				Preservation::Expendable,
			)?;
		}
		Ok(())
	}

	fn resolve_mint(
		recipeint: &AccountId,
		asset: AssetId,
		amount: Balance,
	) -> Result<(), DispatchError> {
		if asset == NativeAssetId::get() {
			return Err(DispatchError::Other("Cannot mint Native Asset"));
		} else {
			Others::mint_into(asset.into(), recipeint, amount.saturated_into())?;
		}
		Ok(())
	}
}

/// Enumerated asset on chain
#[derive(
	Encode,
	Decode,
	Copy,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Ord,
	PartialOrd,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum AssetId {
	/// PDEX the native currency of the chain
	Asset(u128),
	Polkadex,
}

impl AssetId {
	pub fn asset_id(&self) -> Option<u128> {
		match self {
			AssetId::Asset(id) => Some(*id),
			AssetId::Polkadex => None,
		}
	}
}

use sp_runtime::traits::Zero;
impl From<u128> for AssetId {
	fn from(value: u128) -> Self {
		if value.is_zero() {
			Self::Polkadex
		} else {
			Self::Asset(value)
		}
	}
}

pub struct AssetIdConverter;

impl MultiAssetIdConverter<AssetId, u128> for AssetIdConverter {
	fn get_native() -> AssetId {
		AssetId::Polkadex
	}

	fn is_native(asset: &AssetId) -> bool {
		*asset == Self::get_native()
	}

	fn try_convert(asset: &AssetId) -> MultiAssetIdConversionResult<AssetId, u128> {
		match asset {
			AssetId::Polkadex => MultiAssetIdConversionResult::Native,
			AssetId::Asset(id) => MultiAssetIdConversionResult::Converted(*id),
		}
	}
}

impl Serialize for AssetId {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match *self {
			AssetId::Asset(ref id) => {
				serializer.serialize_newtype_variant("asset_id", 0, "asset", &id.to_string())
			},
			AssetId::Polkadex => {
				serializer.serialize_newtype_variant("asset_id", 1, "asset", "PDEX")
			},
		}
	}
}

impl<'de> Deserialize<'de> for AssetId {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_map(AssetId::Polkadex)
	}
}

impl<'de> Visitor<'de> for AssetId {
	type Value = Self;

	fn expecting(&self, formatter: &mut Formatter) -> sp_std::fmt::Result {
		formatter.write_str("expecting an asset id map in the for {\"asset\":\"123\"}")
	}

	fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		// While there are entries remaining in the input, add them
		// into our map.
		while let Some((key, mut value)) = access.next_entry::<String, String>()? {
			if key == *"asset" {
				return if value == *"PDEX" {
					Ok(AssetId::Polkadex)
				} else {
					// Check if its hex or not
					let radix = if value.contains("0x") {
						value = value.replace("0x", "");
						16
					} else {
						10
					};
					match u128::from_str_radix(&value, radix) {
						Err(_) => Err(A::Error::invalid_type(
							Unexpected::Unsigned(128),
							&format!("Expected an u128 string: recv {value:?}").as_str(),
						)),
						Ok(id) => Ok(AssetId::Asset(id)),
					}
				};
			}
		}
		Err(A::Error::invalid_type(Unexpected::Enum, &"Expected an asset id enum"))
	}
}

#[cfg(feature = "std")]
impl TryFrom<String> for AssetId {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		if value.as_str() == "PDEX" {
			return Ok(AssetId::Polkadex);
		}

		match value.parse::<u128>() {
			Ok(id) => Ok(AssetId::Asset(id)),
			Err(_) => {
				Err(anyhow::Error::msg::<String>(format!("Could not parse 'AssetId' from {value}")))
			},
		}
	}
}

impl Display for AssetId {
	fn fmt(&self, f: &mut Formatter<'_>) -> sp_std::fmt::Result {
		match self {
			AssetId::Polkadex => write!(f, "PDEX"),
			AssetId::Asset(id) => write!(f, "{id:?}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::AssetId;

	#[test]
	pub fn test_assetid_serde() {
		let polkadex_asset = AssetId::Polkadex;
		let asset_max = AssetId::Asset(u128::MAX);

		println!("{:?}", serde_json::to_string(&polkadex_asset).unwrap());
		println!("{:?}", serde_json::to_string(&asset_max).unwrap());

		assert_eq!(
			polkadex_asset,
			serde_json::from_str(&serde_json::to_string(&polkadex_asset).unwrap()).unwrap()
		);
		assert_eq!(
			asset_max,
			serde_json::from_str(&serde_json::to_string(&asset_max).unwrap()).unwrap()
		)
	}
}
