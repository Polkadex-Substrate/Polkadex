#![cfg(test)]

use super::*;
use crate::{
    models::{AssetId},
};

pub type AccountId = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const PDEX: AssetId = AssetId::Asset(0x1);
pub const BTC: AssetId = AssetId::Asset(0x2);
pub const DOT: AssetId = AssetId::Asset(0x3);