use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DuplicatePageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Size,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Filetype,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateMeta {
    pub title: String,
    pub media_type: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<crate::dto::Series>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateCandidateRow {
    pub mam_id: u64,
    pub meta: DuplicateMeta,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicateOriginalRow {
    pub id: String,
    pub mam_id: Option<u64>,
    pub meta: DuplicateMeta,
    pub linked: bool,
    pub linked_path: Option<String>,
    pub created_at: String,
    pub abs_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DuplicatePairRow {
    pub torrent: DuplicateCandidateRow,
    pub duplicate_of: DuplicateOriginalRow,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct DuplicateData {
    pub torrents: Vec<DuplicatePairRow>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub abs_url: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateBulkAction {
    Replace,
    Remove,
}

impl DuplicateBulkAction {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Replace => "replace original",
            Self::Remove => "remove duplicate",
        }
    }

    pub(super) fn success_label(self) -> &'static str {
        match self {
            Self::Replace => "Replaced original torrents",
            Self::Remove => "Removed duplicate torrents",
        }
    }
}
