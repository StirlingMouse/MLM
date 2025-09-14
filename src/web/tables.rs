use std::{
    fmt::{self, Display},
    ops::RangeInclusive,
};

use askama::{
    Template,
    filters::{self, HtmlSafe},
};
use serde::{Deserialize, Serialize};

use super::Conditional;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub from: Option<usize>,
    pub page_size: Option<usize>,
}

impl PaginationParams {
    pub fn default_page_size(&self, page_size: usize, total: usize) -> Option<Pagination> {
        let from = self.from.unwrap_or_default();
        let page_size = self.page_size.unwrap_or(page_size);
        if page_size == 0 {
            None
        } else {
            Some(Pagination {
                from,
                page_size,
                total,
                max_pages: 7,
            })
        }
    }
}

/// ```askama
/// {% if page_size > 0 && total > page_size %}
/// <div class=pagination>
///   {% if num_pages() > max_pages %}
///     <a href="?from=0" {% if from == 0 %}class=disabled{% endif %}>«</a>
///   {% endif %}
///   <a href="?from={{ prev() }}" {% if from == 0 %}class=disabled{% endif %}>‹</a>
///   <div>
///   {% for page in pages() %}
///     <a href="?from={{ link(*page) }}" {% if active(*page) %}class=active{% endif %}>{{page}}</a>
///   {% endfor %}
///   </div>
///   <a href="?from={{ next() }}" {% if last() %}class=disabled{% endif %}>›</a>
///   {% if num_pages() > max_pages %}
///     <a href="?from={{ (num_pages() - 1) * self.page_size }}" {% if last() %}class=disabled{% endif %}>»</a>
///   {% endif %}
/// </div>
/// {% endif %}
/// ```
#[derive(Template, Default)]
#[template(ext = "html", in_doc = true)]
pub struct Pagination {
    pub from: usize,
    pub page_size: usize,
    pub total: usize,
    pub max_pages: usize,
}
impl Pagination {
    pub fn num_pages(&self) -> usize {
        (self.total as f64 / self.page_size as f64).ceil().round() as usize
    }
    pub fn pages(&self) -> RangeInclusive<usize> {
        let last_page = self.num_pages();
        if last_page > self.max_pages {
            let current_page = self.from / self.page_size + 1;
            let half = self.max_pages / 2;
            if current_page <= half {
                1..=self.max_pages
            } else if current_page >= last_page - half {
                (last_page - self.max_pages + 1)..=last_page
            } else {
                (current_page - half)..=(current_page + half)
            }
        } else {
            1..=last_page
        }
    }
    pub fn active(&self, page: usize) -> bool {
        let from = (page - 1) * self.page_size;
        from == self.from
    }
    pub fn prev(&self) -> usize {
        self.from.saturating_sub(self.page_size)
    }
    pub fn next(&self) -> usize {
        let last_page = self.num_pages();
        (self.from + self.page_size).min((last_page.saturating_sub(1)) * self.page_size)
    }
    pub fn link(&self, page: usize) -> usize {
        self.page_size * (page - 1)
    }
    pub fn last(&self) -> bool {
        self.from >= self.total - self.page_size
    }
    pub fn selector<'a>(&'a self, values: &'a [usize]) -> PageSizeSelector<'a> {
        PageSizeSelector {
            current: self.page_size,
            values,
        }
    }
}

/// ```askama
/// <select name=page_size>
/// {% for (value, selected) in options() %}
///   <option value="{{value}}" {% if selected %}selected{% endif %}>{{value}}</option>
/// {% endfor %}
///  <option value="0" {% if current == 0 %}selected{% endif %}>all</option>
/// </select>
/// ```
#[derive(Template, Default)]
#[template(ext = "html", in_doc = true)]
pub struct PageSizeSelector<'a> {
    pub current: usize,
    pub values: &'a [usize],
}

impl<'a> PageSizeSelector<'a> {
    pub fn options(&self) -> impl Iterator<Item = (usize, bool)> {
        let mut values = vec![];
        if self.current != 0 && !self.values.contains(&self.current) {
            values.push(self.current);
        }
        for value in self.values {
            values.push(*value);
        }
        values.into_iter().map(|v| (v, v == self.current))
    }
}

#[derive(Clone, Copy, Deserialize)]
pub struct SortOn<T: Key> {
    pub sort_by: Option<T>,
    #[serde(default)]
    pub asc: bool,
}

pub trait Key: Clone + Copy + PartialEq + Serialize {}

impl<T: Key> Display for SortOn<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.sort_by.unwrap().serialize(f)
    }
}

pub trait Sortable {
    type SortKey: Key;

    fn get_current_sort(&self) -> SortOn<Self::SortKey>;

    fn table_header(
        &self,
        sort_key: Option<Self::SortKey>,
        label: &str,
    ) -> TableHeader<Self::SortKey> {
        let sort = self.get_current_sort();
        TableHeader {
            current_key: sort.sort_by,
            asc: sort.asc,
            key: sort_key,
            label: label.to_owned(),
        }
    }
}

pub trait HidableColumns: Sortable {
    fn add_column(&self, size: &str);

    fn table_header_if(
        &self,
        show: &bool,
        sort_key: Option<Self::SortKey>,
        label: &str,
        size: &str,
    ) -> Conditional<TableHeader<Self::SortKey>> {
        let sort = self.get_current_sort();
        if *show {
            self.add_column(size);
        }
        Conditional {
            template: show.then_some(TableHeader {
                current_key: sort.sort_by,
                asc: sort.asc,
                key: sort_key,
                label: label.to_owned(),
            }),
        }
    }
    fn table_header_s(
        &self,
        sort_key: Option<Self::SortKey>,
        label: &str,
        size: &str,
    ) -> TableHeader<Self::SortKey> {
        let sort = self.get_current_sort();
        self.add_column(size);
        TableHeader {
            current_key: sort.sort_by,
            asc: sort.asc,
            key: sort_key,
            label: label.to_owned(),
        }
    }
    fn table_header_all<'a>(&self, name: &'a str, size: &str) -> AllColumnHeader<'a> {
        self.add_column(size);
        AllColumnHeader { name }
    }
}

/// ```askama
/// <div class="header">
/// <input type=checkbox name={{ name }}_all>
/// </div>
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
pub struct AllColumnHeader<'a> {
    name: &'a str,
}

impl<'a> HtmlSafe for AllColumnHeader<'a> {}

/// ```askama
/// {% match key %}
/// {% when Some(key) %}
/// <a
///   href="{{link()}}"
///   class="header {% if Some(**key) == current_key %}sorting{% endif %}"
/// >
/// {{ label }}
/// {% if Some(**key) == current_key %}
///   {% if asc %}↑{% else %}↓{% endif %}
/// {% endif %}
/// </a>
/// {% when None %}
/// <div class="header">
/// {{ label }}
/// </div>
/// {% endmatch %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
pub struct TableHeader<T: Key> {
    current_key: Option<T>,
    asc: bool,
    key: Option<T>,
    label: String,
}
impl<T: Key> HtmlSafe for TableHeader<T> {}

impl<T: Key> TableHeader<T> {
    pub fn link(&self) -> String {
        let key = SortOn {
            sort_by: self.key,
            asc: false,
        };
        if self.key == self.current_key {
            format!("?sort_by={}&asc={}", key, !self.asc)
        } else {
            format!("?sort_by={}", key)
        }
    }
}

pub fn table_styles(cols: u64) -> String {
    table_styles_rows(cols, 1)
}
pub fn table_styles_rows(cols: u64, rows: u64) -> String {
    let mut styles = format!("grid-template-columns: repeat({cols}, auto);");

    let grid = cols * rows;
    for i in 1..=grid {
        styles.push_str(&format!(
            "& > div:nth-child({}n+{})",
            grid * 2,
            grid + i + cols
        ));
        if i < grid {
            styles.push(',');
        }
    }
    styles.push_str("{ background: var(--alternate); }");

    styles
}

/// ```askama
/// <a href="{{link()}}">{{label}}</a>
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
pub struct ItemFilter<'a, T: Key> {
    field: T,
    label: &'a str,
    value: Option<&'a str>,
}
impl<'a, T: Key> HtmlSafe for ItemFilter<'a, T> {}

impl<'a, T: Key> ItemFilter<'a, T> {
    pub fn link(&self) -> String {
        let key = SortOn {
            sort_by: Some(self.field),
            asc: false,
        };
        format!(
            "?{}={}",
            key,
            &urlencoding::encode(self.value.unwrap_or(self.label))
        )
    }
}

pub fn item<T: Key>(field: T, label: &str) -> ItemFilter<T> {
    ItemFilter {
        field,
        label,
        value: None,
    }
}

pub fn item_v<'a, T: Key>(field: T, label: &'a str, value: &'a str) -> ItemFilter<'a, T> {
    ItemFilter {
        field,
        label,
        value: Some(value),
    }
}

/// ```askama
/// {% for label in labels %}
/// {{ self::item(*field, label) | safe }}{% if !loop.last %}, {% endif %}
/// {% endfor %}
/// ```
#[derive(Template)]
#[template(ext = "html", in_doc = true)]
pub struct ItemFilters<'a, T: Key> {
    field: T,
    labels: &'a [String],
}
impl<'a, T: Key> HtmlSafe for ItemFilters<'a, T> {}

pub fn items<'a, T: Key>(field: T, labels: &'a [String]) -> ItemFilters<'a, T> {
    ItemFilters { field, labels }
}
