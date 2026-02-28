use serde::{Deserialize, Serialize};

use crate::components::{
    build_query_string, encode_query_enum, parse_location_query_pairs, parse_query_enum,
};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ErrorsPageSort {
    Step,
    Title,
    Error,
    CreatedAt,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ErrorsPageFilter {
    Step,
    Title,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorsRow {
    pub id_json: String,
    pub step: String,
    pub title: String,
    pub error: String,
    pub created_at: String,
    pub mam_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ErrorsData {
    pub errors: Vec<ErrorsRow>,
}

pub fn filter_name(filter: ErrorsPageFilter) -> &'static str {
    match filter {
        ErrorsPageFilter::Step => "Step",
        ErrorsPageFilter::Title => "Title",
    }
}

#[derive(Clone, Default)]
pub struct PageQueryState {
    pub sort: Option<ErrorsPageSort>,
    pub asc: bool,
    pub filters: Vec<(ErrorsPageFilter, String)>,
}

pub fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<ErrorsPageSort>(&value),
            "asc" => state.asc = value == "true",
            _ => {
                if let Some(field) = parse_query_enum::<ErrorsPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

pub fn build_query_url(
    sort: Option<ErrorsPageSort>,
    asc: bool,
    filters: &[(ErrorsPageFilter, String)],
) -> String {
    let mut params = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}
