use serde::{Deserialize, Serialize};

use crate::components::PageColumns;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageSort {
    Kind,
    Title,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Cost,
    Buffer,
    Grabber,
    CreatedAt,
    StartedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedPageFilter {
    Kind,
    Category,
    Flags,
    Title,
    Author,
    Narrator,
    Series,
    Language,
    Filetype,
    Cost,
    Grabber,
}

#[derive(Clone, Copy)]
pub enum SelectedColumn {
    Category,
    Flags,
    Authors,
    Narrators,
    Series,
    Language,
    Size,
    Filetypes,
    Grabber,
    CreatedAt,
    StartedAt,
    RemovedAt,
}

pub const COLUMN_OPTIONS: &[(SelectedColumn, &str)] = &[
    (SelectedColumn::Category, "Category"),
    (SelectedColumn::Flags, "Flags"),
    (SelectedColumn::Authors, "Authors"),
    (SelectedColumn::Narrators, "Narrators"),
    (SelectedColumn::Series, "Series"),
    (SelectedColumn::Language, "Language"),
    (SelectedColumn::Size, "Size"),
    (SelectedColumn::Filetypes, "Filetypes"),
    (SelectedColumn::Grabber, "Grabber"),
    (SelectedColumn::CreatedAt, "Added At"),
    (SelectedColumn::StartedAt, "Started At"),
    (SelectedColumn::RemovedAt, "Removed At"),
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedPageColumns {
    pub category: bool,
    pub flags: bool,
    pub authors: bool,
    pub narrators: bool,
    pub series: bool,
    pub language: bool,
    pub size: bool,
    pub filetypes: bool,
    pub grabber: bool,
    pub created_at: bool,
    pub started_at: bool,
    pub removed_at: bool,
}

impl Default for SelectedPageColumns {
    fn default() -> Self {
        Self {
            category: false,
            flags: false,
            authors: true,
            narrators: false,
            series: true,
            language: false,
            size: true,
            filetypes: true,
            grabber: true,
            created_at: true,
            started_at: true,
            removed_at: false,
        }
    }
}

impl SelectedPageColumns {
    pub fn table_grid_template(self) -> String {
        let mut cols = vec!["30px", if self.category { "130px" } else { "84px" }];
        if self.flags {
            cols.push("60px");
        }
        cols.push("2fr");
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
        cols.push("80px");
        cols.push("120px");
        if self.grabber {
            cols.push("130px");
        }
        if self.created_at {
            cols.push("157px");
        }
        if self.started_at {
            cols.push("157px");
        }
        if self.removed_at {
            cols.push("157px");
        }
        cols.join(" ")
    }

    pub fn get(self, col: SelectedColumn) -> bool {
        match col {
            SelectedColumn::Category => self.category,
            SelectedColumn::Flags => self.flags,
            SelectedColumn::Authors => self.authors,
            SelectedColumn::Narrators => self.narrators,
            SelectedColumn::Series => self.series,
            SelectedColumn::Language => self.language,
            SelectedColumn::Size => self.size,
            SelectedColumn::Filetypes => self.filetypes,
            SelectedColumn::Grabber => self.grabber,
            SelectedColumn::CreatedAt => self.created_at,
            SelectedColumn::StartedAt => self.started_at,
            SelectedColumn::RemovedAt => self.removed_at,
        }
    }

    pub fn set(&mut self, col: SelectedColumn, enabled: bool) {
        match col {
            SelectedColumn::Category => self.category = enabled,
            SelectedColumn::Flags => self.flags = enabled,
            SelectedColumn::Authors => self.authors = enabled,
            SelectedColumn::Narrators => self.narrators = enabled,
            SelectedColumn::Series => self.series = enabled,
            SelectedColumn::Language => self.language = enabled,
            SelectedColumn::Size => self.size = enabled,
            SelectedColumn::Filetypes => self.filetypes = enabled,
            SelectedColumn::Grabber => self.grabber = enabled,
            SelectedColumn::CreatedAt => self.created_at = enabled,
            SelectedColumn::StartedAt => self.started_at = enabled,
            SelectedColumn::RemovedAt => self.removed_at = enabled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedMeta {
    pub title: String,
    pub media_type: String,
    pub cat_name: String,
    pub cat_id: Option<String>,
    pub flags: Vec<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<crate::dto::Series>,
    pub language: Option<String>,
    pub size: String,
    pub filetypes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedRow {
    pub mam_id: u64,
    pub meta: SelectedMeta,
    pub cost: String,
    pub required_unsats: u64,
    pub grabber: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub removed_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SelectedUserInfo {
    pub unsat_count: u64,
    pub unsat_limit: u64,
    pub wedges: u64,
    pub bonus: i64,
    pub remaining_buffer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct SelectedData {
    pub torrents: Vec<SelectedRow>,
    pub queued: usize,
    pub downloading: usize,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SelectedBulkAction {
    Remove,
    Update,
}

impl SelectedBulkAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Remove => "unselect for download",
            Self::Update => "set required unsats to",
        }
    }

    pub fn success_label(self) -> &'static str {
        match self {
            Self::Remove => "Updated selected torrents",
            Self::Update => "Updated required unsats",
        }
    }
}

pub fn filter_name(filter: SelectedPageFilter) -> &'static str {
    match filter {
        SelectedPageFilter::Kind => "Type",
        SelectedPageFilter::Category => "Category",
        SelectedPageFilter::Flags => "Flags",
        SelectedPageFilter::Title => "Title",
        SelectedPageFilter::Author => "Authors",
        SelectedPageFilter::Narrator => "Narrators",
        SelectedPageFilter::Series => "Series",
        SelectedPageFilter::Language => "Language",
        SelectedPageFilter::Filetype => "Filetypes",
        SelectedPageFilter::Cost => "Cost",
        SelectedPageFilter::Grabber => "Grabber",
    }
}

impl PageColumns for SelectedPageColumns {
    fn to_query_value(&self) -> String {
        let mut values = Vec::new();
        if self.category {
            values.push("category");
        }
        if self.flags {
            values.push("flags");
        }
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
        if self.grabber {
            values.push("grabber");
        }
        if self.created_at {
            values.push("created_at");
        }
        if self.started_at {
            values.push("started_at");
        }
        if self.removed_at {
            values.push("removed_at");
        }
        values.join(",")
    }

    fn from_query_value(value: &str) -> Self {
        let mut show = SelectedPageColumns {
            category: false,
            flags: false,
            authors: false,
            narrators: false,
            series: false,
            language: false,
            size: false,
            filetypes: false,
            grabber: false,
            created_at: false,
            started_at: false,
            removed_at: false,
        };
        for item in value.split(',') {
            match item {
                "category" => show.category = true,
                "flags" => show.flags = true,
                "author" => show.authors = true,
                "narrator" => show.narrators = true,
                "series" => show.series = true,
                "language" => show.language = true,
                "size" => show.size = true,
                "filetype" => show.filetypes = true,
                "grabber" => show.grabber = true,
                "created_at" => show.created_at = true,
                "started_at" => show.started_at = true,
                "removed_at" => show.removed_at = true,
                _ => {}
            }
        }
        show
    }
}
