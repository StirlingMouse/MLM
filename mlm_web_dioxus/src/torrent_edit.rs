use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::error::{IntoServerFnError, OptionIntoServerFnError};
#[cfg(feature = "server")]
use mlm_core::ContextExt;
#[cfg(feature = "server")]
use mlm_db::DatabaseExt;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TorrentMetaEditForm {
    pub torrent_id: String,
    pub ids_text: String,
    pub vip_mode: String,
    pub vip_temp_date: String,
    pub category_id: String,
    pub media_type_id: String,
    pub main_cat_id: String,
    pub categories_text: String,
    pub tags_text: String,
    pub language_id: String,
    pub crude_language: bool,
    pub violence: bool,
    pub some_explicit: bool,
    pub explicit: bool,
    pub abridged: bool,
    pub lgbt: bool,
    pub filetypes_text: String,
    pub num_files: String,
    pub size: String,
    pub title: String,
    pub edition: String,
    pub edition_number: String,
    pub description: String,
    pub authors_text: String,
    pub narrators_text: String,
    pub series_text: String,
    pub source: String,
    pub uploaded_at_unix: String,
}

#[cfg(feature = "server")]
fn split_list(text: &str) -> Vec<String> {
    text.lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(feature = "server")]
fn parse_series(text: &str) -> Result<Vec<mlm_db::Series>, ServerFnError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split_once(" #")
                .map(|(name, entries)| {
                    mlm_db::Series::try_from((name.to_string(), entries.to_string()))
                })
                .unwrap_or_else(|| mlm_db::Series::try_from((line.to_string(), String::new())))
                .map_err(|e| ServerFnError::new(format!("failed to parse series '{line}': {e}")))
        })
        .collect()
}

#[cfg(feature = "server")]
fn parse_ids(text: &str) -> Result<std::collections::BTreeMap<String, String>, ServerFnError> {
    let mut ids = std::collections::BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ServerFnError::new(format!(
                "invalid ids line '{line}', expected key=value"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            return Err(ServerFnError::new(format!(
                "invalid ids line '{line}', key and value must be non-empty"
            )));
        }
        ids.insert(key.to_string(), value.to_string());
    }
    Ok(ids)
}

#[cfg(feature = "server")]
fn parse_vip_status(
    mode: &str,
    temp_date: &str,
) -> Result<Option<mlm_db::VipStatus>, ServerFnError> {
    let mode = mode.trim().to_lowercase();
    match mode.as_str() {
        "" | "none" => Ok(None),
        "not_vip" => Ok(Some(mlm_db::VipStatus::NotVip)),
        "permanent" => Ok(Some(mlm_db::VipStatus::Permanent)),
        "temp" => {
            let value = temp_date.trim();
            if value.is_empty() {
                return Err(ServerFnError::new(
                    "vip temp date is required when vip mode is temp",
                ));
            }
            let date_format = time::format_description::parse("[year]-[month]-[day]")
                .map_err(|e| ServerFnError::new(format!("invalid vip date format config: {e}")))?;
            let date = time::Date::parse(value, &date_format).map_err(|e| {
                ServerFnError::new(format!("failed to parse vip temp date '{value}': {e}"))
            })?;
            Ok(Some(mlm_db::VipStatus::Temp(date)))
        }
        _ => Err(ServerFnError::new(format!("invalid vip mode '{mode}'"))),
    }
}

#[server]
pub async fn get_torrent_meta_edit_data(id: String) -> Result<TorrentMetaEditForm, ServerFnError> {
    use itertools::Itertools;

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
    let (vip_mode, vip_temp_date) = match meta.vip_status {
        None => ("none".to_string(), String::new()),
        Some(mlm_db::VipStatus::NotVip) => ("not_vip".to_string(), String::new()),
        Some(mlm_db::VipStatus::Permanent) => ("permanent".to_string(), String::new()),
        Some(mlm_db::VipStatus::Temp(date)) => ("temp".to_string(), date.to_string()),
    };

    Ok(TorrentMetaEditForm {
        torrent_id: id,
        ids_text: meta.ids.iter().map(|(k, v)| format!("{k}={v}")).join("\n"),
        vip_mode,
        vip_temp_date,
        category_id: meta
            .cat
            .map(|cat: mlm_db::OldCategory| cat.as_id().to_string())
            .unwrap_or_default(),
        media_type_id: meta.media_type.as_id().to_string(),
        main_cat_id: meta
            .main_cat
            .map(|cat: mlm_db::MainCat| cat.as_id().to_string())
            .unwrap_or_default(),
        categories_text: meta.categories.join("\n"),
        tags_text: meta.tags.join("\n"),
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
        filetypes_text: meta.filetypes.join("\n"),
        num_files: meta.num_files.to_string(),
        size: meta.size.to_string(),
        title: meta.title,
        edition: meta
            .edition
            .as_ref()
            .map(|(ed, _): &(String, u64)| ed.clone())
            .unwrap_or_default(),
        edition_number: meta
            .edition
            .as_ref()
            .map(|(_, idx): &(String, u64)| idx.to_string())
            .unwrap_or_default(),
        description: meta.description,
        authors_text: meta.authors.join("\n"),
        narrators_text: meta.narrators.join("\n"),
        series_text: meta
            .series
            .iter()
            .map(mlm_db::impls::format_serie)
            .join("\n"),
        source: match meta.source {
            mlm_db::MetadataSource::Mam => "mam".to_string(),
            mlm_db::MetadataSource::Manual => "manual".to_string(),
            mlm_db::MetadataSource::File => "file".to_string(),
            mlm_db::MetadataSource::Match => "match".to_string(),
        },
        uploaded_at_unix: meta.uploaded_at.0.unix_timestamp().to_string(),
    })
}

#[server]
pub async fn update_torrent_meta_edit_data(form: TorrentMetaEditForm) -> Result<(), ServerFnError> {
    let context = crate::error::get_context()?;

    let config = context.config().await;
    let torrent = context
        .db()
        .r_transaction()
        .server_err()?
        .get()
        .primary::<mlm_db::Torrent>(form.torrent_id.clone())
        .server_err()?
        .ok_or_server_err("Torrent not found")?;

    let ids = parse_ids(&form.ids_text)?;
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

    let source = match form.source.trim().to_lowercase().as_str() {
        "mam" => mlm_db::MetadataSource::Mam,
        "manual" => mlm_db::MetadataSource::Manual,
        "file" => mlm_db::MetadataSource::File,
        "match" => mlm_db::MetadataSource::Match,
        value => return Err(ServerFnError::new(format!("invalid source '{value}'"))),
    };

    let uploaded_at_unix = form
        .uploaded_at_unix
        .trim()
        .parse::<i64>()
        .map_err(|e| ServerFnError::new(format!("invalid uploaded_at unix timestamp: {e}")))?;
    let uploaded_at = time::UtcDateTime::from_unix_timestamp(uploaded_at_unix)
        .map_err(|e| ServerFnError::new(format!("invalid uploaded_at unix timestamp: {e}")))?;

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

    let meta =
        mlm_db::TorrentMeta {
            ids,
            vip_status: parse_vip_status(&form.vip_mode, &form.vip_temp_date)?,
            cat: category,
            media_type,
            main_cat,
            categories: split_list(&form.categories_text),
            tags: split_list(&form.tags_text),
            language,
            flags: Some(mlm_db::FlagBits::new(flags.as_bitfield())),
            filetypes: split_list(&form.filetypes_text),
            num_files: form
                .num_files
                .trim()
                .parse::<u64>()
                .map_err(|e| ServerFnError::new(format!("invalid num_files: {e}")))?,
            size: form.size.trim().parse::<mlm_db::Size>().map_err(|e| {
                ServerFnError::new(format!("invalid size '{}': {e}", form.size.trim()))
            })?,
            title: form.title.trim().to_string(),
            edition,
            description: form.description,
            authors: split_list(&form.authors_text),
            narrators: split_list(&form.narrators_text),
            series: parse_series(&form.series_text)?,
            source,
            uploaded_at: uploaded_at.into(),
        };

    mlm_core::autograbber::update_torrent_meta(
        &config,
        context.db(),
        context.db().rw_async().await.server_err()?,
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
pub fn TorrentEditPage(id: String) -> Element {
    let mut status_msg = use_signal(|| None::<(String, bool)>);
    let mut form_state = use_signal(|| None::<TorrentMetaEditForm>);

    let data_res = use_server_future(move || {
        let id = id.clone();
        async move { get_torrent_meta_edit_data(id).await }
    })?;

    if let Some(Ok(data)) = &*data_res.value().read()
        && form_state.read().as_ref() != Some(data)
    {
        form_state.set(Some(data.clone()));
    }

    rsx! {
        div { class: "torrent-edit-page",
            h1 { "Edit Torrent Metadata" }

            if let Some((msg, is_error)) = status_msg.read().as_ref() {
                p { class: if *is_error { "error" } else { "loading-indicator" }, "{msg}" }
            }

            if let Some(form) = form_state.read().as_ref().cloned() {
                form {
                    class: "column",
                    onsubmit: move |ev: Event<FormData>| {
                        ev.prevent_default();
                        let current = form_state.read().clone();
                        let Some(payload) = current else {
                            return;
                        };
                        spawn(async move {
                            match update_torrent_meta_edit_data(payload).await {
                                Ok(_) => status_msg.set(Some(("Metadata updated".to_string(), false))),
                                Err(e) => status_msg.set(Some((format!("Update failed: {e}"), true))),
                            }
                        });
                    },

                    label {
                        "Title"
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

                    label {
                        "Description"
                        textarea {
                            rows: "6",
                            value: "{form.description}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.description = ev.value();
                                }
                            },
                        }
                    }

                    label {
                        "IDs (key=value per line)"
                        textarea {
                            rows: "5",
                            value: "{form.ids_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.ids_text = ev.value();
                                }
                            },
                        }
                    }

                    div { class: "row",
                        label {
                            "VIP Mode"
                            select {
                                value: "{form.vip_mode}",
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.vip_mode = ev.value();
                                    }
                                },
                                option { value: "none", "None" }
                                option { value: "not_vip", "Not VIP" }
                                option { value: "permanent", "Permanent" }
                                option { value: "temp", "Temporary" }
                            }
                        }
                        label {
                            "VIP Temp Date (YYYY-MM-DD)"
                            input {
                                r#type: "text",
                                value: "{form.vip_temp_date}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.vip_temp_date = ev.value();
                                    }
                                },
                            }
                        }
                    }

                    div { class: "row",
                        label {
                            "Category ID"
                            input {
                                r#type: "text",
                                value: "{form.category_id}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.category_id = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Media Type ID"
                            input {
                                r#type: "text",
                                value: "{form.media_type_id}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.media_type_id = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Main Category ID"
                            input {
                                r#type: "text",
                                value: "{form.main_cat_id}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.main_cat_id = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Language ID"
                            input {
                                r#type: "text",
                                value: "{form.language_id}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.language_id = ev.value();
                                    }
                                },
                            }
                        }
                    }

                    div { class: "row",
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.crude_language,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.crude_language = ev.value() == "true";
                                    }
                                },
                            }
                            "Crude language"
                        }
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.violence,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.violence = ev.value() == "true";
                                    }
                                },
                            }
                            "Violence"
                        }
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.some_explicit,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.some_explicit = ev.value() == "true";
                                    }
                                },
                            }
                            "Some explicit"
                        }
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.explicit,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.explicit = ev.value() == "true";
                                    }
                                },
                            }
                            "Explicit"
                        }
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.abridged,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.abridged = ev.value() == "true";
                                    }
                                },
                            }
                            "Abridged"
                        }
                        label {
                            input {
                                r#type: "checkbox",
                                checked: form.lgbt,
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.lgbt = ev.value() == "true";
                                    }
                                },
                            }
                            "LGBT"
                        }
                    }

                    label {
                        "Categories (newline/comma separated)"
                        textarea {
                            rows: "4",
                            value: "{form.categories_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.categories_text = ev.value();
                                }
                            },
                        }
                    }

                    label {
                        "Tags (newline/comma separated)"
                        textarea {
                            rows: "4",
                            value: "{form.tags_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.tags_text = ev.value();
                                }
                            },
                        }
                    }

                    label {
                        "Filetypes (newline/comma separated)"
                        textarea {
                            rows: "3",
                            value: "{form.filetypes_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.filetypes_text = ev.value();
                                }
                            },
                        }
                    }

                    div { class: "row",
                        label {
                            "Num Files"
                            input {
                                r#type: "text",
                                value: "{form.num_files}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.num_files = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Size"
                            input {
                                r#type: "text",
                                value: "{form.size}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.size = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Uploaded At (unix seconds)"
                            input {
                                r#type: "text",
                                value: "{form.uploaded_at_unix}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.uploaded_at_unix = ev.value();
                                    }
                                },
                            }
                        }
                    }

                    div { class: "row",
                        label {
                            "Edition"
                            input {
                                r#type: "text",
                                value: "{form.edition}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.edition = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Edition Number"
                            input {
                                r#type: "text",
                                value: "{form.edition_number}",
                                oninput: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.edition_number = ev.value();
                                    }
                                },
                            }
                        }
                        label {
                            "Source"
                            select {
                                value: "{form.source}",
                                onchange: move |ev| {
                                    if let Some(state) = form_state.write().as_mut() {
                                        state.source = ev.value();
                                    }
                                },
                                option { value: "mam", "MaM" }
                                option { value: "manual", "Manual" }
                                option { value: "file", "File" }
                                option { value: "match", "Match" }
                            }
                        }
                    }

                    label {
                        "Authors (newline/comma separated)"
                        textarea {
                            rows: "4",
                            value: "{form.authors_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.authors_text = ev.value();
                                }
                            },
                        }
                    }

                    label {
                        "Narrators (newline/comma separated)"
                        textarea {
                            rows: "4",
                            value: "{form.narrators_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.narrators_text = ev.value();
                                }
                            },
                        }
                    }

                    label {
                        "Series (one per line, format: Name #1)"
                        textarea {
                            rows: "4",
                            value: "{form.series_text}",
                            oninput: move |ev| {
                                if let Some(state) = form_state.write().as_mut() {
                                    state.series_text = ev.value();
                                }
                            },
                        }
                    }

                    div { class: "row",
                        button { r#type: "submit", class: "btn", "Save" }
                        a { class: "btn", href: "/dioxus/torrents/{form.torrent_id}", "Back to Torrent" }
                    }
                }
            } else if let Some(Err(e)) = &*data_res.value().read() {
                p { class: "error", "Error: {e}" }
            } else {
                p { "Loading torrent metadata..." }
            }
        }
    }
}
