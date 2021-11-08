#![cfg(test)]

use crate::{
    models::{AssetId},
};

pub const PDEX: AssetId = AssetId::Asset(0x1);
pub const BTC: AssetId = AssetId::Asset(0x2);
pub const DOT: AssetId = AssetId::Asset(0x3);