use crate::components::{
    PageColumns, build_query_string, encode_query_enum, parse_location_query_pairs,
    parse_query_enum,
};

use super::types::{SelectedPageColumns, SelectedPageFilter, SelectedPageSort};

#[derive(Clone, Default)]
pub struct PageQueryState {
    pub sort: Option<SelectedPageSort>,
    pub asc: bool,
    pub filters: Vec<(SelectedPageFilter, String)>,
    pub show: SelectedPageColumns,
}

pub fn parse_query_state() -> PageQueryState {
    let mut state = PageQueryState::default();
    for (key, value) in parse_location_query_pairs() {
        match key.as_str() {
            "sort_by" => state.sort = parse_query_enum::<SelectedPageSort>(&value),
            "asc" => state.asc = value == "true",
            "show" => state.show = SelectedPageColumns::from_query_value(&value),
            _ => {
                if let Some(field) = parse_query_enum::<SelectedPageFilter>(&key) {
                    state.filters.push((field, value));
                }
            }
        }
    }
    state
}

pub fn build_query_url(
    sort: Option<SelectedPageSort>,
    asc: bool,
    filters: &[(SelectedPageFilter, String)],
    show: SelectedPageColumns,
) -> String {
    let mut params = Vec::new();
    if let Some(sort) = sort.and_then(encode_query_enum) {
        params.push(("sort_by".to_string(), sort));
    }
    if asc {
        params.push(("asc".to_string(), "true".to_string()));
    }
    if show != SelectedPageColumns::default() {
        params.push(("show".to_string(), show.to_query_value()));
    }
    for (field, value) in filters {
        if let Some(name) = encode_query_enum(*field) {
            params.push((name, value.clone()));
        }
    }
    build_query_string(&params)
}
