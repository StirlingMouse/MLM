use crate::components::{
    PageColumns, build_query_string, encode_query_enum, parse_location_query_pairs,
    parse_query_enum,
};

use super::{TorrentsPageColumns, TorrentsPageFilter, TorrentsPageSort};

type ColumnKey = (
    &'static str,
    fn(&TorrentsPageColumns) -> bool,
    fn(&mut TorrentsPageColumns, bool),
);

#[derive(Clone)]
pub(super) struct PageQueryState {
    pub(super) query: String,
    pub(super) sort: Option<TorrentsPageSort>,
    pub(super) asc: bool,
    pub(super) filters: Vec<(TorrentsPageFilter, String)>,
    pub(super) from: usize,
    pub(super) page_size: usize,
    pub(super) show: TorrentsPageColumns,
}

impl Default for PageQueryState {
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

/// Single source of truth mapping each column to its (query_key, getter, setter).
/// Adding a new column requires only adding one entry here.
const COLUMN_KEYS: &[ColumnKey] = &[
    (
        "category",
        |c| c.category,
        |c, v| {
            c.category = v;
        },
    ),
    (
        "categories",
        |c| c.categories,
        |c, v| {
            c.categories = v;
        },
    ),
    (
        "flags",
        |c| c.flags,
        |c, v| {
            c.flags = v;
        },
    ),
    (
        "edition",
        |c| c.edition,
        |c, v| {
            c.edition = v;
        },
    ),
    (
        "author",
        |c| c.authors,
        |c, v| {
            c.authors = v;
        },
    ),
    (
        "narrator",
        |c| c.narrators,
        |c, v| {
            c.narrators = v;
        },
    ),
    (
        "series",
        |c| c.series,
        |c, v| {
            c.series = v;
        },
    ),
    (
        "language",
        |c| c.language,
        |c, v| {
            c.language = v;
        },
    ),
    (
        "size",
        |c| c.size,
        |c, v| {
            c.size = v;
        },
    ),
    (
        "filetype",
        |c| c.filetypes,
        |c, v| {
            c.filetypes = v;
        },
    ),
    (
        "linker",
        |c| c.linker,
        |c, v| {
            c.linker = v;
        },
    ),
    (
        "qbit_category",
        |c| c.qbit_category,
        |c, v| {
            c.qbit_category = v;
        },
    ),
    (
        "path",
        |c| c.path,
        |c, v| {
            c.path = v;
        },
    ),
    (
        "created_at",
        |c| c.created_at,
        |c, v| {
            c.created_at = v;
        },
    ),
    (
        "uploaded_at",
        |c| c.uploaded_at,
        |c, v| {
            c.uploaded_at = v;
        },
    ),
];

impl PageColumns for TorrentsPageColumns {
    fn to_query_value(&self) -> String {
        COLUMN_KEYS
            .iter()
            .filter(|(_, get, _)| get(self))
            .map(|(key, _, _)| *key)
            .collect::<Vec<_>>()
            .join(",")
    }

    fn from_query_value(value: &str) -> Self {
        let mut cols = TorrentsPageColumns {
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
            if let Some((_, _, set)) = COLUMN_KEYS.iter().find(|(key, _, _)| *key == item) {
                set(&mut cols, true);
            }
        }
        cols
    }
}

pub(super) fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
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
                state.show = TorrentsPageColumns::from_query_value(&value);
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

pub(super) fn build_query_url(
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
        params.push(("show".to_string(), show.to_query_value()));
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
