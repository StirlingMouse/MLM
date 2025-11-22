use anyhow::{Error, Result};
use htmlentity::entity::{self, ICodedDataTrait as _};
use unidecode::unidecode;

#[derive(thiserror::Error, Debug)]
pub enum MetaError {
    #[error("{0}")]
    UnknownMediaType(String),
    #[error("Unknown category: {0}")]
    UnknownCat(u64),
    #[error("Unknown old category: {0}")]
    UnknownOldCat(String),
    #[error("Unknown language id {0}, code: {1}")]
    UnknownLanguage(u8, String),
    #[error("{0}")]
    InvalidSize(String),
    #[error("{0}")]
    InvalidSeries(&'static str),
    #[error("Invalid vip_expiry: {0}")]
    InvalidVipExpiry(u64),
    #[error("Unknown error: {0}")]
    Other(#[from] Error),
}

pub fn clean_value(value: &str) -> Result<String> {
    entity::decode(value.as_bytes()).to_string()
}

pub fn normalize_title(value: &str) -> String {
    unidecode(value).to_lowercase()
}
