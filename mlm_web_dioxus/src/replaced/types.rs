use crate::components::PageColumns;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ReplacedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Replaced,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReplacedPageFilter {
    Kind,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Linked,
}

#[derive(Clone, Copy)]
pub enum ReplacedColumn {
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Filetypes,
}

pub const COLUMN_OPTIONS: &[(ReplacedColumn, &str)] = &[
    (ReplacedColumn::Authors, "Authors"),
    (ReplacedColumn::Narrators, "Narrators"),
    (ReplacedColumn::Series, "Series"),
    (ReplacedColumn::Language, "Language"),
    (ReplacedColumn::Size, "Size"),
    (ReplacedColumn::Filetypes, "Filetypes"),
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedPageColumns {
    pub authors: bool,
    pub narrators: bool,
    pub series: bool,
    pub language: bool,
    pub size: bool,
    pub filetypes: bool,
}

impl Default for ReplacedPageColumns {
    fn default() -> Self {
        Self {
            authors: true,
            narrators: true,
            series: true,
            language: false,
            size: true,
            filetypes: true,
        }
    }
}

impl ReplacedPageColumns {
    pub fn table_grid_template(self) -> String {
        let mut cols = vec!["30px", "110px", "2fr"];
        if self.authors {
            cols.push("1fr");
        }
        if self.narrators {
            cols.push("1fr");
        }
        if self.series {
            cols.push("1fr");
        }
        if self.language {
            cols.push("100px");
        }
        if self.size {
            cols.push("81px");
        }
        if self.filetypes {
            cols.push("100px");
        }
        cols.push("157px");
        cols.push("157px");
        cols.push("132px");
        cols.join(" ")
    }

    pub fn get(self, col: ReplacedColumn) -> bool {
        match col {
            ReplacedColumn::Authors => self.authors,
            ReplacedColumn::Narrators => self.narrators,
            ReplacedColumn::Series => self.series,
            ReplacedColumn::Language => self.language,
            ReplacedColumn::Size => self.size,
            ReplacedColumn::Filetypes => self.filetypes,
        }
    }

    pub fn set(&mut self, col: ReplacedColumn, enabled: bool) {
        match col {
            ReplacedColumn::Authors => self.authors = enabled,
            ReplacedColumn::Narrators => self.narrators = enabled,
            ReplacedColumn::Series => self.series = enabled,
            ReplacedColumn::Language => self.language = enabled,
            ReplacedColumn::Size => self.size = enabled,
            ReplacedColumn::Filetypes => self.filetypes = enabled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedMeta {
    pub title: String,
    pub media_type: String,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<crate::dto::Series>,
    pub language: Option<String>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedRow {
    pub id: String,
    pub mam_id: Option<u64>,
    pub meta: ReplacedMeta,
    pub linked: bool,
    pub created_at: String,
    pub replaced_at: Option<String>,
    pub abs_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplacedPairRow {
    pub torrent: ReplacedRow,
    pub replacement: ReplacedRow,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ReplacedData {
    pub torrents: Vec<ReplacedPairRow>,
    pub total: usize,
    pub from: usize,
    pub page_size: usize,
    pub abs_url: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ReplacedBulkAction {
    Refresh,
    RefreshRelink,
    Remove,
}

impl ReplacedBulkAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Refresh => "refresh metadata",
            Self::RefreshRelink => "refresh metadata and relink",
            Self::Remove => "remove torrent from MLM",
        }
    }

    pub fn success_label(self) -> &'static str {
        match self {
            Self::Refresh => "Refreshed metadata",
            Self::RefreshRelink => "Refreshed metadata and relinked",
            Self::Remove => "Removed torrents",
        }
    }
}

impl PageColumns for ReplacedPageColumns {
    fn to_query_value(&self) -> String {
        let mut values = Vec::new();
        if self.authors {
            values.push("author");
        }
        if self.narrators {
            values.push("narrator");
        }
        if self.series {
            values.push("series");
        }
        if self.language {
            values.push("language");
        }
        if self.size {
            values.push("size");
        }
        if self.filetypes {
            values.push("filetype");
        }
        values.join(",")
    }

    fn from_query_value(value: &str) -> Self {
        let mut show = ReplacedPageColumns {
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
        };
        for item in value.split(',') {
            match item {
                "author" => show.authors = true,
                "narrator" => show.narrators = true,
                "series" => show.series = true,
                "language" => show.language = true,
                "size" => show.size = true,
                "filetype" => show.filetypes = true,
                _ => {}
            }
        }
        show
    }
}
