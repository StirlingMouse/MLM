use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::flag_icon;

#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_db::DatabaseExt;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SeriesEditRow {
    pub name: String,
    pub entries: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SelectOptionData {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TorrentMetaEditForm {
    pub torrent_id: String,
    pub abs_id: String,
    pub asin: String,
    pub goodreads_id: String,
    pub mam_id: String,
    pub category_id: String,
    pub media_type_id: String,
    pub main_cat_id: String,
    pub language_id: String,
    pub crude_language: bool,
    pub violence: bool,
    pub some_explicit: bool,
    pub explicit: bool,
    pub abridged: bool,
    pub lgbt: bool,
    pub title: String,
    pub edition: String,
    pub edition_number: String,
    pub description: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub series: Vec<SeriesEditRow>,
    pub vip_status_label: String,
    pub filetypes: Vec<String>,
    pub num_files: u64,
    pub size_label: String,
    pub uploaded_at_label: String,
    pub legacy_category_options: Vec<SelectOptionData>,
    pub media_type_options: Vec<SelectOptionData>,
    pub main_category_options: Vec<SelectOptionData>,
    pub language_options: Vec<SelectOptionData>,
    pub category_suggestions: Vec<String>,
}

#[derive(Clone, PartialEq, Props)]
struct MultiValueEditorProps {
    label: String,
    helper: String,
    input_label: String,
    placeholder: String,
    empty_label: String,
    selected: Vec<String>,
    suggestions: Vec<String>,
    allow_custom: bool,
    on_add: EventHandler<String>,
    on_remove: EventHandler<String>,
}

#[derive(Clone, PartialEq, Props)]
struct SeriesEditorProps {
    rows: Vec<SeriesEditRow>,
    on_add_row: EventHandler<()>,
    on_update_name: EventHandler<(usize, String)>,
    on_update_entries: EventHandler<(usize, String)>,
    on_remove_row: EventHandler<usize>,
}

#[derive(Clone, PartialEq, Props)]
struct FlagToggleCardProps {
    label: String,
    flag_key: String,
    checked: bool,
    on_toggle: EventHandler<bool>,
}

#[cfg(feature = "server")]
fn clean_list(items: &[String]) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for item in items {
        let value = item.trim();
        if value.is_empty() {
            continue;
        }
        if !cleaned
            .iter()
            .any(|existing| normalize_value(existing) == normalize_value(value))
        {
            cleaned.push(value.to_string());
        }
    }
    cleaned
}

#[cfg(feature = "server")]
fn parse_series(rows: &[SeriesEditRow]) -> Result<Vec<mlm_db::Series>, ServerFnError> {
    rows.iter()
        .filter_map(|row| {
            let name = row.name.trim();
            let entries = row.entries.trim();
            if name.is_empty() && entries.is_empty() {
                None
            } else {
                Some((name.to_string(), entries.to_string()))
            }
        })
        .map(|(name, entries)| {
            if name.is_empty() {
                return Err(ServerFnError::new(
                    "series name is required when series entries are provided",
                ));
            }

            mlm_db::Series::try_from((name.clone(), entries))
                .map_err(|e| ServerFnError::new(format!("failed to parse series '{name}': {e}")))
        })
        .collect()
}

#[cfg(feature = "server")]
fn upsert_known_id(ids: &mut std::collections::BTreeMap<String, String>, key: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        ids.remove(key);
    } else {
        ids.insert(key.to_string(), value.to_string());
    }
}

fn normalize_value(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn add_unique_value(values: &mut Vec<String>, value: String) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }

    let normalized = normalize_value(trimmed);
    if values
        .iter()
        .any(|existing| normalize_value(existing) == normalized)
    {
        return;
    }

    values.push(trimmed.to_string());
}

fn remove_value(values: &mut Vec<String>, target: &str) {
    let normalized = normalize_value(target);
    values.retain(|value| normalize_value(value) != normalized);
}

#[server]
pub async fn get_torrent_meta_edit_data(id: String) -> Result<TorrentMetaEditForm, ServerFnError> {
    fn legacy_category_group(category: &mlm_db::OldCategory) -> &'static str {
        match category {
            mlm_db::OldCategory::Audio(_) => "Audiobook",
            mlm_db::OldCategory::Ebook(_) => "Ebook",
            mlm_db::OldCategory::Musicology(_) => "Musicology",
            mlm_db::OldCategory::Radio(_) => "Radio",
        }
    }

    let context = crate::error::get_context()?;

    let torrent = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<mlm_db::Torrent>(id.clone())
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let meta = torrent.meta;
    let flags = mlm_db::Flags::from_bitfield(meta.flags.map_or(0, |f| f.0));

    Ok(TorrentMetaEditForm {
        torrent_id: id,
        abs_id: meta.ids.get(mlm_db::ids::ABS).cloned().unwrap_or_default(),
        asin: meta.ids.get(mlm_db::ids::ASIN).cloned().unwrap_or_default(),
        goodreads_id: meta
            .ids
            .get(mlm_db::ids::GOODREADS)
            .cloned()
            .unwrap_or_default(),
        mam_id: meta.ids.get(mlm_db::ids::MAM).cloned().unwrap_or_default(),
        category_id: meta
            .cat
            .map(|cat: mlm_db::OldCategory| cat.as_id().to_string())
            .unwrap_or_default(),
        media_type_id: meta.media_type.as_id().to_string(),
        main_cat_id: meta
            .main_cat
            .map(|cat: mlm_db::MainCat| cat.as_id().to_string())
            .unwrap_or_default(),
        language_id: meta
            .language
            .map(|language: mlm_db::Language| language.to_id().to_string())
            .unwrap_or_default(),
        crude_language: flags.crude_language.unwrap_or(false),
        violence: flags.violence.unwrap_or(false),
        some_explicit: flags.some_explicit.unwrap_or(false),
        explicit: flags.explicit.unwrap_or(false),
        abridged: flags.abridged.unwrap_or(false),
        lgbt: flags.lgbt.unwrap_or(false),
        title: meta.title,
        edition: meta
            .edition
            .as_ref()
            .map(|(edition, _)| edition.clone())
            .unwrap_or_default(),
        edition_number: meta
            .edition
            .as_ref()
            .map(|(_, number)| number.to_string())
            .unwrap_or_default(),
        description: meta.description,
        categories: meta
            .categories
            .iter()
            .map(|category| category.as_str().to_string())
            .collect(),
        tags: meta.tags,
        authors: meta.authors,
        narrators: meta.narrators,
        series: meta
            .series
            .into_iter()
            .map(|series| SeriesEditRow {
                name: series.name,
                entries: series.entries.to_string(),
            })
            .collect(),
        vip_status_label: meta
            .vip_status
            .map(|status| status.to_string())
            .unwrap_or_else(|| "Not set".to_string()),
        filetypes: meta.filetypes,
        num_files: meta.num_files,
        size_label: meta.size.to_string(),
        uploaded_at_label: meta
            .uploaded_at
            .map(|uploaded_at| uploaded_at.0.to_string())
            .unwrap_or_else(|| "Not available".to_string()),
        legacy_category_options: mlm_db::OldCategory::all()
            .into_iter()
            .map(|category: mlm_db::OldCategory| SelectOptionData {
                value: category.as_id().to_string(),
                label: format!(
                    "{} - {}",
                    legacy_category_group(&category),
                    category.as_str()
                ),
            })
            .collect(),
        media_type_options: mlm_db::MediaType::all()
            .iter()
            .map(|media_type| SelectOptionData {
                value: media_type.as_id().to_string(),
                label: media_type.as_str().to_string(),
            })
            .collect(),
        main_category_options: mlm_db::MainCat::all()
            .into_iter()
            .map(|main_cat: mlm_db::MainCat| SelectOptionData {
                value: main_cat.as_id().to_string(),
                label: main_cat.as_str().to_string(),
            })
            .collect(),
        language_options: mlm_db::Language::all()
            .into_iter()
            .map(|language: mlm_db::Language| SelectOptionData {
                value: language.to_id().to_string(),
                label: language.to_str().to_string(),
            })
            .collect(),
        category_suggestions: mlm_db::Category::all()
            .into_iter()
            .map(|category: mlm_db::Category| category.as_str().to_string())
            .collect(),
    })
}

#[server]
pub async fn update_torrent_meta_edit_data(form: TorrentMetaEditForm) -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;

    let config = context.config().await;
    let rw = context.db().rw_async().await.server_err()?;
    let torrent =
        rw.1.get()
            .primary::<mlm_db::Torrent>(form.torrent_id.clone())
            .server_err()?
            .ok_or_server_err("Torrent not found")?;

    let category = if form.category_id.trim().is_empty() {
        None
    } else {
        let value = form
            .category_id
            .trim()
            .parse::<u64>()
            .map_err(|e| ServerFnError::new(format!("invalid category id: {e}")))?;
        mlm_db::OldCategory::from_one_id(value)
            .ok_or_server_err(&format!("unknown category id {value}"))?
            .into()
    };

    let media_type = if form.media_type_id.trim().is_empty() {
        if let Some(category) = category.as_ref() {
            category.as_main_cat().into()
        } else {
            torrent.meta.media_type
        }
    } else {
        let value = form
            .media_type_id
            .trim()
            .parse::<u8>()
            .map_err(|e| ServerFnError::new(format!("invalid media type id: {e}")))?;
        mlm_db::MediaType::from_id(value)
            .ok_or_server_err(&format!("unknown media type id {value}"))?
    };

    let main_cat = if form.main_cat_id.trim().is_empty() {
        None
    } else {
        let value = form
            .main_cat_id
            .trim()
            .parse::<u8>()
            .map_err(|e| ServerFnError::new(format!("invalid main category id: {e}")))?;
        mlm_db::MainCat::from_id(value)
            .ok_or_server_err(&format!("unknown main category id {value}"))?
            .into()
    };

    let language = if form.language_id.trim().is_empty() {
        None
    } else {
        let value = form
            .language_id
            .trim()
            .parse::<u8>()
            .map_err(|e| ServerFnError::new(format!("invalid language id: {e}")))?;
        Some(
            mlm_db::Language::from_id(value)
                .ok_or_server_err(&format!("unknown language id {value}"))?,
        )
    };

    let edition = if form.edition.trim().is_empty() {
        None
    } else {
        let number = if form.edition_number.trim().is_empty() {
            0
        } else {
            form.edition_number
                .trim()
                .parse::<u64>()
                .map_err(|e| ServerFnError::new(format!("invalid edition number: {e}")))?
        };
        Some((form.edition.trim().to_string(), number))
    };

    let flags = mlm_db::Flags {
        crude_language: Some(form.crude_language),
        violence: Some(form.violence),
        some_explicit: Some(form.some_explicit),
        explicit: Some(form.explicit),
        abridged: Some(form.abridged),
        lgbt: Some(form.lgbt),
    };

    let categories = clean_list(&form.categories)
        .into_iter()
        .map(|raw| {
            raw.parse::<mlm_db::Category>()
                .map_err(|e| ServerFnError::new(format!("invalid category '{raw}': {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut ids = torrent.meta.ids.clone();
    upsert_known_id(&mut ids, mlm_db::ids::ABS, &form.abs_id);
    upsert_known_id(&mut ids, mlm_db::ids::ASIN, &form.asin);
    upsert_known_id(&mut ids, mlm_db::ids::GOODREADS, &form.goodreads_id);
    upsert_known_id(&mut ids, mlm_db::ids::MAM, &form.mam_id);

    let mut meta = torrent.meta.clone();
    meta.ids = ids;
    meta.cat = category;
    meta.media_type = media_type;
    meta.main_cat = main_cat;
    meta.categories = categories;
    meta.tags = clean_list(&form.tags);
    meta.language = language;
    meta.flags = Some(mlm_db::FlagBits::new(flags.as_bitfield()));
    meta.title = form.title.trim().to_string();
    meta.edition = edition;
    meta.description = form.description;
    meta.authors = clean_list(&form.authors);
    meta.narrators = clean_list(&form.narrators);
    meta.series = parse_series(&form.series)?;
    meta.source = mlm_db::MetadataSource::Manual;

    mlm_core::autograbber::update_torrent_meta(
        &config,
        context.db(),
        rw,
        None,
        torrent,
        meta,
        true,
        false,
        &context.events,
    )
    .await
    .server_err()?;

    Ok(())
}

#[component]
fn MultiValueEditor(props: MultiValueEditorProps) -> Element {
    let mut query = use_signal(String::new);
    let trimmed_query = query.read().trim().to_string();
    let normalized_query = normalize_value(&trimmed_query);

    let mut filtered_suggestions = if normalized_query.is_empty() {
        Vec::new()
    } else {
        props
            .suggestions
            .iter()
            .filter(|item| {
                let normalized_item = normalize_value(item);
                !props
                    .selected
                    .iter()
                    .any(|selected| normalize_value(selected) == normalized_item)
                    && normalized_item.contains(&normalized_query)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    filtered_suggestions.sort_by(|left, right| {
        let left_key = normalize_value(left);
        let right_key = normalize_value(right);
        let left_starts = !left_key.starts_with(&normalized_query);
        let right_starts = !right_key.starts_with(&normalized_query);
        left_starts
            .cmp(&right_starts)
            .then_with(|| left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase()))
    });
    filtered_suggestions.truncate(8);

    let can_add_custom = props.allow_custom
        && !trimmed_query.is_empty()
        && !props
            .selected
            .iter()
            .any(|selected| normalize_value(selected) == normalized_query);

    let add_text = if let Some(first_match) = filtered_suggestions.first() {
        first_match.clone()
    } else {
        trimmed_query.clone()
    };
    let has_filtered_suggestions = !filtered_suggestions.is_empty();
    let suggestions_for_keyboard = filtered_suggestions.clone();
    let suggestions_for_button = filtered_suggestions.clone();

    rsx! {
        div { class: "multi-value-editor",
            div { class: "editor-header",
                div {
                    h3 { class: "editor-title", "{props.label}" }
                    p { class: "editor-helper", "{props.helper}" }
                }
            }

            div { class: "editor-selected",
                if props.selected.is_empty() {
                    p { class: "editor-empty", "{props.empty_label}" }
                } else {
                    for value in props.selected.iter().cloned() {
                        div { class: "editor-chip",
                            span { class: "editor-chip-label", "{value}" }
                            button {
                                r#type: "button",
                                class: "editor-chip-remove",
                                "aria-label": "Remove {value}",
                                onclick: move |_| props.on_remove.call(value.clone()),
                                span { "×" }
                            }
                        }
                    }
                }
            }

            label { class: "field",
                span { class: "field-label sr-only", "{props.input_label}" }
                div { class: "editor-input-row",
                    input {
                        r#type: "text",
                        "aria-label": "{props.input_label}",
                        class: "editor-input",
                        value: "{query}",
                        placeholder: "{props.placeholder}",
                        oninput: move |ev| query.set(ev.value()),
                        onkeydown: move |ev| match ev.key() {
                            Key::Enter => {
                                ev.prevent_default();
                                if let Some(first_match) = suggestions_for_keyboard.first() {
                                    props.on_add.call(first_match.clone());
                                    query.set(String::new());
                                } else if can_add_custom {
                                    props.on_add.call(trimmed_query.clone());
                                    query.set(String::new());
                                }
                            }
                            Key::Escape => query.set(String::new()),
                            Key::Backspace if trimmed_query.is_empty() => {
                                if let Some(last_value) = props.selected.last() {
                                    props.on_remove.call(last_value.clone());
                                }
                            }
                            _ => {}
                        },
                    }
                    button {
                        r#type: "button",
                        class: "btn btn-secondary",
                        disabled: !can_add_custom && !has_filtered_suggestions,
                        onclick: move |_| {
                            if let Some(first_match) = suggestions_for_button.first() {
                                props.on_add.call(first_match.clone());
                                query.set(String::new());
                            } else if can_add_custom {
                                props.on_add.call(add_text.clone());
                                query.set(String::new());
                            }
                        },
                        "Add"
                    }
                }
            }

            if !filtered_suggestions.is_empty() {
                div { class: "editor-suggestions",
                    p { class: "editor-suggestions-label", "Suggestions" }
                    div { class: "editor-suggestion-list",
                        for suggestion in filtered_suggestions {
                            button {
                                key: "{suggestion}",
                                r#type: "button",
                                class: "editor-suggestion",
                                onclick: move |_| {
                                    props.on_add.call(suggestion.clone());
                                    query.set(String::new());
                                },
                                "{suggestion}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SeriesEditor(props: SeriesEditorProps) -> Element {
    rsx! {
        div { class: "series-editor",
            div { class: "editor-header",
                div {
                    h3 { class: "editor-title", "Series" }
                    p { class: "editor-helper",
                        "Add one row per series. Entries can be a number, range, or part like 1, 1-3, or 2p1."
                    }
                }
                button {
                    r#type: "button",
                    class: "btn btn-secondary",
                    onclick: move |_| props.on_add_row.call(()),
                    "Add series"
                }
            }

            if props.rows.is_empty() {
                p { class: "editor-empty", "No series added yet." }
            } else {
                div { class: "series-list",
                    for (index , row) in props.rows.iter().enumerate() {
                        div {
                            class: "series-row",
                            key: "{index}-{row.name}-{row.entries}",
                            label { class: "field",
                                span { class: "field-label", "Series name" }
                                input {
                                    r#type: "text",
                                    value: "{row.name}",
                                    placeholder: "The Stormlight Archive",
                                    oninput: move |ev| {
                                        props.on_update_name.call((index, ev.value()));
                                    },
                                }
                            }
                            label { class: "field",
                                span { class: "field-label", "Entries" }
                                input {
                                    r#type: "text",
                                    value: "{row.entries}",
                                    placeholder: "1, 2-3, 4p1",
                                    oninput: move |ev| {
                                        props.on_update_entries.call((index, ev.value()));
                                    },
                                }
                            }
                            button {
                                r#type: "button",
                                class: "editor-icon-button",
                                "aria-label": "Remove series row {index}",
                                onclick: move |_| props.on_remove_row.call(index),
                                span { "×" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FlagToggleCard(props: FlagToggleCardProps) -> Element {
    let icon = flag_icon(&props.flag_key);

    rsx! {
        label { class: "flag-card",
            input {
                r#type: "checkbox",
                checked: props.checked,
                onchange: move |ev| props.on_toggle.call(ev.checked()),
            }
            if let Some((src, title)) = icon {
                img {
                    class: "flag-card-icon flag",
                    src: "{src}",
                    alt: "{title}",
                    title: "{title}",
                }
            }
            span { class: "flag-card-label", "{props.label}" }
        }
    }
}

#[component]
pub fn TorrentEditPage(id: String) -> Element {
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut form_state = use_signal(|| None::<TorrentMetaEditForm>);
    let mut loaded_form = use_signal(|| None::<TorrentMetaEditForm>);

    let data_res = use_server_future(move || {
        let id = id.clone();
        async move { get_torrent_meta_edit_data(id).await }
    })?;

    use_effect(move || {
        if let Some(Ok(data)) = &*data_res.value().read()
            && loaded_form.read().as_ref() != Some(data)
        {
            loaded_form.set(Some(data.clone()));
            form_state.set(Some(data.clone()));
        }
    });

    rsx! {
        div { class: "torrent-edit-page",
            div { class: "torrent-edit-shell",
                div { class: "torrent-edit-hero",
                    div {
                        h1 { "Edit Torrent Metadata" }
                    }
                }

                if let Some((msg, is_error)) = status_msg.read().as_ref() {
                    p { class: if *is_error { "torrent-edit-banner error" } else { "torrent-edit-banner" },
                        "{msg}"
                    }
                }

                if let Some(form) = form_state.read().as_ref().cloned() {
                    form {
                        class: "torrent-edit-form",
                        onsubmit: move |ev: Event<FormData>| {
                            ev.prevent_default();
                            let current = form_state.read().clone();
                            let Some(payload) = current else {
                                return;
                            };

                            status_msg.set(Some(("Saving metadata...".to_string(), false)));
                            spawn(async move {
                                match update_torrent_meta_edit_data(payload.clone()).await {
                                    Ok(_) => {
                                        match get_torrent_meta_edit_data(payload.torrent_id.clone()).await {
                                            Ok(refreshed) => {
                                                loaded_form.set(Some(refreshed.clone()));
                                                form_state.set(Some(refreshed));
                                            }
                                            Err(e) => {
                                                status_msg
                                                    .set(
                                                        Some((
                                                            format!("Metadata updated, but refresh failed: {e}"),
                                                            true,
                                                        )),
                                                    );
                                                return;
                                            }
                                        }
                                        status_msg.set(Some(("Metadata updated".to_string(), false)));
                                    }
                                    Err(e) => {
                                        status_msg.set(Some((format!("Update failed: {e}"), true)));
                                    }
                                }
                            });
                        },

                        section { class: "torrent-edit-section",
                            div { class: "section-heading",
                                h2 { "Title & Summary" }
                            }
                            div { class: "torrent-edit-grid torrent-edit-grid-wide",
                                label { class: "field field-span-2",
                                    span { class: "field-label", "Title" }
                                    input {
                                        r#type: "text",
                                        value: "{form.title}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.title = ev.value();
                                            }
                                        },
                                    }
                                }

                                label { class: "field",
                                    span { class: "field-label", "Edition" }
                                    input {
                                        r#type: "text",
                                        value: "{form.edition}",
                                        placeholder: "Unabridged, Anniversary, Collector's Edition...",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.edition = ev.value();
                                            }
                                        },
                                    }
                                }

                                label { class: "field",
                                    span { class: "field-label", "Edition number" }
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        value: "{form.edition_number}",
                                        placeholder: "1",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.edition_number = ev.value();
                                            }
                                        },
                                    }
                                }

                                label { class: "field field-span-2",
                                    span { class: "field-label", "Description" }
                                    textarea {
                                        rows: "10",
                                        value: "{form.description}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.description = ev.value();
                                            }
                                        },
                                    }
                                }
                            }
                        }

                        section { class: "torrent-edit-section",
                            div { class: "section-heading",
                                h2 { "Identifiers" }
                            }
                            div { class: "torrent-edit-grid",
                                label { class: "field",
                                    span { class: "field-label", "ABS ID" }
                                    input {
                                        r#type: "text",
                                        value: "{form.abs_id}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.abs_id = ev.value();
                                            }
                                        },
                                    }
                                }
                                label { class: "field",
                                    span { class: "field-label", "ASIN" }
                                    input {
                                        r#type: "text",
                                        value: "{form.asin}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.asin = ev.value();
                                            }
                                        },
                                    }
                                }
                                label { class: "field",
                                    span { class: "field-label", "Goodreads ID" }
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        value: "{form.goodreads_id}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.goodreads_id = ev.value();
                                            }
                                        },
                                    }
                                }
                                label { class: "field",
                                    span { class: "field-label", "MaM ID" }
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        value: "{form.mam_id}",
                                        oninput: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.mam_id = ev.value();
                                            }
                                        },
                                    }
                                }
                            }
                        }

                        section { class: "torrent-edit-section",
                            div { class: "section-heading",
                                h2 { "Classification" }
                            }
                            div { class: "torrent-edit-grid",
                                label { class: "field",
                                    span { class: "field-label", "Legacy category" }
                                    select {
                                        onchange: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.category_id = ev.value();
                                            }
                                        },
                                        option {
                                            value: "",
                                            selected: form.category_id.is_empty(),
                                            "None"
                                        }
                                        for option_data in form.legacy_category_options.iter() {
                                            option {
                                                value: "{option_data.value}",
                                                selected: option_data.value == form.category_id,
                                                "{option_data.label}"
                                            }
                                        }
                                    }
                                }

                                label { class: "field",
                                    span { class: "field-label", "Media type" }
                                    select {
                                        onchange: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.media_type_id = ev.value();
                                            }
                                        },
                                        option {
                                            value: "",
                                            selected: form.media_type_id.is_empty(),
                                            "Select media type"
                                        }
                                        for option_data in form.media_type_options.iter() {
                                            option {
                                                value: "{option_data.value}",
                                                selected: option_data.value == form.media_type_id,
                                                "{option_data.label}"
                                            }
                                        }
                                    }
                                }

                                label { class: "field",
                                    span { class: "field-label", "Main category" }
                                    select {
                                        onchange: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.main_cat_id = ev.value();
                                            }
                                        },
                                        option {
                                            value: "",
                                            selected: form.main_cat_id.is_empty(),
                                            "Not set"
                                        }
                                        for option_data in form.main_category_options.iter() {
                                            option {
                                                value: "{option_data.value}",
                                                selected: option_data.value == form.main_cat_id,
                                                "{option_data.label}"
                                            }
                                        }
                                    }
                                }

                                label { class: "field",
                                    span { class: "field-label", "Language" }
                                    select {
                                        onchange: move |ev| {
                                            if let Some(state) = form_state.write().as_mut() {
                                                state.language_id = ev.value();
                                            }
                                        },
                                        option {
                                            value: "",
                                            selected: form.language_id.is_empty(),
                                            "Not set"
                                        }
                                        for option_data in form.language_options.iter() {
                                            option {
                                                value: "{option_data.value}",
                                                selected: option_data.value == form.language_id,
                                                "{option_data.label}"
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "torrent-edit-stack",
                                MultiValueEditor {
                                    label: "Categories".to_string(),
                                    helper: "Search the available category list, press Enter to add the highlighted match, and Backspace to remove the last chip when the input is empty."
                                        .to_string(),
                                    input_label: "Add category".to_string(),
                                    placeholder: "Search categories...",
                                    empty_label: "No categories selected yet.".to_string(),
                                    selected: form.categories.clone(),
                                    suggestions: form.category_suggestions.clone(),
                                    allow_custom: false,
                                    on_add: move |value| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            add_unique_value(&mut state.categories, value);
                                        }
                                    },
                                    on_remove: move |value: String| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            remove_value(&mut state.categories, &value);
                                        }
                                    },
                                }

                                MultiValueEditor {
                                    label: "Tags".to_string(),
                                    helper: "Use short tags for details that do not belong in the structured categories above."
                                        .to_string(),
                                    input_label: "Add tag".to_string(),
                                    placeholder: "Add a tag and press Enter",
                                    empty_label: "No tags selected yet.".to_string(),
                                    selected: form.tags.clone(),
                                    suggestions: Vec::new(),
                                    allow_custom: true,
                                    on_add: move |value| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            add_unique_value(&mut state.tags, value);
                                        }
                                    },
                                    on_remove: move |value: String| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            remove_value(&mut state.tags, &value);
                                        }
                                    },
                                }
                            }
                        }

                        section { class: "torrent-edit-section",
                            div { class: "section-heading",
                                h2 { "Contributors & Series" }
                            }
                            div { class: "torrent-edit-stack",
                                MultiValueEditor {
                                    label: "Authors".to_string(),
                                    helper: "".to_string(),
                                    input_label: "Add author".to_string(),
                                    placeholder: "Add an author",
                                    empty_label: "No authors added yet.".to_string(),
                                    selected: form.authors.clone(),
                                    suggestions: Vec::new(),
                                    allow_custom: true,
                                    on_add: move |value| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            add_unique_value(&mut state.authors, value);
                                        }
                                    },
                                    on_remove: move |value: String| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            remove_value(&mut state.authors, &value);
                                        }
                                    },
                                }

                                MultiValueEditor {
                                    label: "Narrators".to_string(),
                                    helper: "".to_string(),
                                    input_label: "Add narrator".to_string(),
                                    placeholder: "Add a narrator",
                                    empty_label: "No narrators added yet.".to_string(),
                                    selected: form.narrators.clone(),
                                    suggestions: Vec::new(),
                                    allow_custom: true,
                                    on_add: move |value| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            add_unique_value(&mut state.narrators, value);
                                        }
                                    },
                                    on_remove: move |value: String| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            remove_value(&mut state.narrators, &value);
                                        }
                                    },
                                }

                                SeriesEditor {
                                    rows: form.series.clone(),
                                    on_add_row: move |_| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.series.push(SeriesEditRow::default());
                                        }
                                    },
                                    on_update_name: move |(index, value): (usize, String)| {
                                        if let Some(state) = form_state.write().as_mut()
                                            && let Some(row) = state.series.get_mut(index)
                                        {
                                            row.name = value;
                                        }
                                    },
                                    on_update_entries: move |(index, value): (usize, String)| {
                                        if let Some(state) = form_state.write().as_mut()
                                            && let Some(row) = state.series.get_mut(index)
                                        {
                                            row.entries = value;
                                        }
                                    },
                                    on_remove_row: move |index| {
                                        if let Some(state) = form_state.write().as_mut() && index < state.series.len() {
                                            state.series.remove(index);
                                        }
                                    },
                                }
                            }
                        }

                        section { class: "torrent-edit-section",
                            div { class: "section-heading",
                                h2 { "Flags" }
                            }
                            div { class: "flag-grid",
                                FlagToggleCard {
                                    label: "Crude language".to_string(),
                                    flag_key: "language".to_string(),
                                    checked: form.crude_language,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.crude_language = checked;
                                        }
                                    },
                                }
                                FlagToggleCard {
                                    label: "Violence".to_string(),
                                    flag_key: "violence".to_string(),
                                    checked: form.violence,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.violence = checked;
                                        }
                                    },
                                }
                                FlagToggleCard {
                                    label: "Some explicit".to_string(),
                                    flag_key: "some_explicit".to_string(),
                                    checked: form.some_explicit,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.some_explicit = checked;
                                        }
                                    },
                                }
                                FlagToggleCard {
                                    label: "Explicit".to_string(),
                                    flag_key: "explicit".to_string(),
                                    checked: form.explicit,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.explicit = checked;
                                        }
                                    },
                                }
                                FlagToggleCard {
                                    label: "Abridged".to_string(),
                                    flag_key: "abridged".to_string(),
                                    checked: form.abridged,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.abridged = checked;
                                        }
                                    },
                                }
                                FlagToggleCard {
                                    label: "LGBT".to_string(),
                                    flag_key: "lgbt".to_string(),
                                    checked: form.lgbt,
                                    on_toggle: move |checked| {
                                        if let Some(state) = form_state.write().as_mut() {
                                            state.lgbt = checked;
                                        }
                                    },
                                }
                            }
                        }

                        section { class: "torrent-edit-section torrent-edit-section-muted",
                            div { class: "section-heading",
                                h2 { "Internal Metadata" }
                            }
                            div { class: "readonly-grid",
                                div { class: "readonly-card",
                                    span { class: "readonly-label", "VIP status" }
                                    strong { "{form.vip_status_label}" }
                                }
                                div { class: "readonly-card",
                                    span { class: "readonly-label", "File count" }
                                    strong { "{form.num_files}" }
                                }
                                div { class: "readonly-card",
                                    span { class: "readonly-label", "Size" }
                                    strong { "{form.size_label}" }
                                }
                                div { class: "readonly-card",
                                    span { class: "readonly-label", "Uploaded at" }
                                    strong { "{form.uploaded_at_label}" }
                                }
                                div { class: "readonly-card readonly-card-wide",
                                    span { class: "readonly-label", "File types" }
                                    if form.filetypes.is_empty() {
                                        p { class: "editor-empty", "No file types recorded." }
                                    } else {
                                        div { class: "readonly-chip-list",
                                            for filetype in form.filetypes.iter() {
                                                span { class: "pill", "{filetype}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "torrent-edit-actions",
                            button { r#type: "submit", class: "btn", "Save" }
                            a {
                                class: "btn btn-secondary",
                                href: "/torrents/{form.torrent_id}",
                                "Back to Torrent"
                            }
                        }
                    }
                } else if let Some(Err(e)) = &*data_res.value().read() {
                    p { class: "error", "Error: {e}" }
                } else {
                    p { class: "loading-indicator", "Loading torrent metadata..." }
                }
            }
        }
    }
}
