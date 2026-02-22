use serde::Serialize;

use crate::components::{build_query_string, parse_location_query_pairs, parse_query_enum};

use super::{TorrentsPageColumns, TorrentsPageFilter, TorrentsPageSort};

#[derive(Clone)]
pub(super) struct LegacyQueryState {
    pub(super) query: String,
    pub(super) sort: Option<TorrentsPageSort>,
    pub(super) asc: bool,
    pub(super) filters: Vec<(TorrentsPageFilter, String)>,
    pub(super) from: usize,
    pub(super) page_size: usize,
    pub(super) show: TorrentsPageColumns,
}

impl Default for LegacyQueryState {
    fn default() -> Self {
        Self {
            query: String::new(),
            sort: None,
            asc: false,
            filters: Vec::new(),
            from: 0,
            page_size: 500,
            show: TorrentsPageColumns::default(),
        }
    }
}

fn encode_query_enum<T: Serialize>(value: T) -> Option<String> {
    serde_json::to_string(&value)
        .ok()
        .map(|raw| raw.trim_matches('"').to_string())
}

fn show_to_query_value(show: TorrentsPageColumns) -> String {
    let mut values = Vec::new();
    if show.category {
        values.push("category");
    }
    if show.categories {
        values.push("categories");
    }
    if show.flags {
        values.push("flags");
    }
    if show.edition {
        values.push("edition");
    }
    if show.authors {
        values.push("author");
    }
    if show.narrators {
        values.push("narrator");
    }
    if show.series {
        values.push("series");
    }
    if show.language {
        values.push("language");
    }
    if show.size {
        values.push("size");
    }
    if show.filetypes {
        values.push("filetype");
    }
    if show.linker {
        values.push("linker");
    }
    if show.qbit_category {
        values.push("qbit_category");
    }
    if show.path {
        values.push("path");
    }
    if show.created_at {
        values.push("created_at");
    }
    if show.uploaded_at {
        values.push("uploaded_at");
    }
    values.join(",")
}

fn show_from_query_value(value: &str) -> TorrentsPageColumns {
    let mut show = TorrentsPageColumns {
        category: false,
        categories: false,
        flags: false,
        edition: false,
        authors: false,
        narrators: false,
        series: false,
        language: false,
        size: false,
        filetypes: false,
        linker: false,
        qbit_category: false,
        path: false,
        created_at: false,
        uploaded_at: false,
    };
    for item in value.split(',') {
        match item {
            "category" => show.category = true,
            "categories" => show.categories = true,
            "flags" => show.flags = true,
            "edition" => show.edition = true,
            "author" => show.authors = true,
            "narrator" => show.narrators = true,
            "series" => show.series = true,
            "language" => show.language = true,
            "size" => show.size = true,
            "filetype" => show.filetypes = true,
            "linker" => show.linker = true,
            "qbit_category" => show.qbit_category = true,
            "path" => show.path = true,
            "created_at" => show.created_at = true,
            "uploaded_at" => show.uploaded_at = true,
            _ => {}
        }
    }
    show
}

pub(super) fn parse_legacy_query_state() -> LegacyQueryState {
    let mut state = LegacyQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => {
                state.sort = parse_query_enum::<TorrentsPageSort>(&value);
            }
            "asc" => {
                state.asc = value == "true";
            }
            "from" => {
                if let Ok(v) = value.parse::<usize>() {
                    state.from = v;
                }
            }
            "page_size" => {
                if let Ok(v) = value.parse::<usize>() {
                    state.page_size = v;
                }
            }
            "show" => {
                state.show = show_from_query_value(&value);
            }
            "query" => {
                state.query = value;
            }
            _ => {
                if let Some(field) = parse_query_enum::<TorrentsPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

pub(super) fn build_legacy_query_string(
    query: &str,
    sort: Option<TorrentsPageSort>,
    asc: bool,
    filters: &[(TorrentsPageFilter, String)],
    from: usize,
    page_size: usize,
    show: TorrentsPageColumns,
) -> String {
    let mut params: Vec<(String, String)> = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    if from > 0 {
        params.push(("from".to_string(), from.to_string()));
    }
    if page_size != 500 {
        params.push(("page_size".to_string(), page_size.to_string()));
    }
    if show != TorrentsPageColumns::default() {
        params.push(("show".to_string(), show_to_query_value(show)));
    }
    if !query.is_empty() {
        params.push(("query".to_string(), query.to_string()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}
