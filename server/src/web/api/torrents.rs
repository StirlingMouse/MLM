use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use axum::{Json, extract::State};
use native_db::Database;
use serde_json::json;
use time::macros::utc_datetime;

use crate::{
    config::Config,
    data::{Category, Torrent, TorrentKey},
    web::{AppError, MaMState},
};

pub async fn torrents_api(
    State((config, db, mam)): State<(Arc<Config>, Arc<Database<'static>>, MaMState)>,
) -> std::result::Result<Json<serde_json::Value>, AppError> {
    let r = db.r_transaction()?;

    let torrents = r.scan().secondary::<Torrent>(TorrentKey::created_at)?;
    let torrents = torrents.all()?;
    // let start = utc_datetime!(2025 - 10 - 01 00 : 00);
    // let torrents = torrents.filter(|t| {
    //     let Ok(t) = t else {
    //         return true;
    //     };
    //
    //     t.meta.uploaded_at.0 >= start
    // });
    // let mut all_buddies = BTreeSet::new();
    // all_buddies.insert(Some("Oriel"));
    // all_buddies.insert(Some("stormybaby13"));
    // all_buddies.insert(Some("annbland"));
    // all_buddies.insert(Some("naranga"));
    // all_buddies.insert(Some("myxdvz"));
    // all_buddies.insert(Some("helpfulkitten"));
    // all_buddies.insert(Some("doobiegirl"));
    // // let torrents = torrents.collect::<Result<Vec<_>, _>>()?;
    // let mut uploaded = BTreeMap::new();
    // let mut uploaders = BTreeMap::new();
    // let mut buddies = BTreeMap::new();
    // for t in torrents {
    //     let t = t?;
    //     uploaded
    //         .entry(t.meta.uploaded_at.0.date().to_string())
    //         .and_modify(|e| *e += 1)
    //         .or_insert(1);
    //     uploaders
    //         .entry(t.meta.uploaded_at.0.date().to_string())
    //         .and_modify(|e: &mut BTreeSet<String>| {
    //             e.insert(t.linker.clone().unwrap_or_default());
    //         })
    //         .or_insert_with(|| {
    //             let mut set = BTreeSet::new();
    //             set.insert(t.linker.clone().unwrap_or_default());
    //             set
    //         });
    //     if all_buddies.contains(&t.linker.as_deref()) {
    //         buddies
    //             .entry(t.meta.uploaded_at.0.date().to_string())
    //             .and_modify(|e| *e += 1)
    //             .or_insert(1);
    //     }
    // }
    // let uploaders = uploaders
    //     .into_iter()
    //     .map(|(k, v)| (k, v.len()))
    //     .collect::<BTreeMap<_, _>>();
    //
    // Ok::<_, AppError>(Json(json!({
    //     // "torrents": torrents,
    //     "uploaded": uploaded,
    //     "uploaders": uploaders,
    //     "buddies": buddies
    // })))

    let mut uploaded = BTreeMap::new();
    let mut uploaders = BTreeMap::new();
    // let mut size = BTreeMap::new();
    // let mut total_size = 0u64;
    // let mut categories = BTreeMap::new();
    // let mut uploads = BTreeMap::new();
    // let mut uploaders = BTreeSet::new();
    // let mut new_uploaders = BTreeMap::new();

    // for id in 1..=60 {
    //     categories.insert(Category::from_id(id).unwrap(), 0);
    // }

    // let mut m4b = BTreeMap::new();
    // let mut mp3 = BTreeMap::new();
    // let mut epub = BTreeMap::new();
    // let mut mobi = BTreeMap::new();
    // let M4B = "m4b".to_string();
    // let MP3 = "mp3".to_string();
    // let EPUB = "epub".to_string();
    // let MOBI = "mobi".to_string();
    for t in torrents {
        let t = t?;
        let key = (
            t.meta.uploaded_at.0.date().year(),
            u8::from(t.meta.uploaded_at.0.date().month()),
        );
        if t.linker.as_deref() == Some("Goomer") {
            uploaded.entry(key).and_modify(|e| *e += 1).or_insert(1);
        }
        if t.linker.as_deref() == Some("FastSquash") {
            uploaders.entry(key).and_modify(|e| *e += 1).or_insert(1);
        }
        // uploaded.entry(key).and_modify(|e| *e += 1).or_insert(1);
        // uploaders
        //     .entry(key)
        //     .and_modify(|e: &mut BTreeSet<String>| {
        //         e.insert(t.linker.clone().unwrap_or_default());
        //     })
        //     .or_insert_with(|| {
        //         let mut set = BTreeSet::new();
        //         set.insert(t.linker.clone().unwrap_or_default());
        //         set
        //     });
        // size.entry(key)
        //     .and_modify(|e| *e += t.meta.size.bytes())
        //     .or_insert(t.meta.size.bytes());
        // total_size += t.meta.size.bytes();
        // for cat in t.meta.categories.iter() {
        //     categories.entry(*cat).and_modify(|e| *e += 1).or_insert(1);
        // }
        // let is_new = uploaders.insert(t.linker.clone().unwrap_or_default());
        // if is_new {
        //     new_uploaders
        //         .entry(key)
        //         .and_modify(|e| *e += 1)
        //         .or_insert(1);
        // }
        // if key == (2022, 2) {
        //     uploads
        //         .entry(t.linker.clone().unwrap_or_default())
        //         .and_modify(|e| *e += 1)
        //         .or_insert(1);
        // }

        // if t.meta.filetypes.contains(&M4B) {
        //     m4b.entry(key).and_modify(|e| *e += 1).or_insert(1);
        // }
        // if t.meta.filetypes.contains(&MP3) {
        //     mp3.entry(key).and_modify(|e| *e += 1).or_insert(1);
        // }
        // if t.meta.filetypes.contains(&EPUB) {
        //     epub.entry(key).and_modify(|e| *e += 1).or_insert(1);
        // }
        // if t.meta.filetypes.contains(&MOBI) {
        //     mobi.entry(key).and_modify(|e| *e += 1).or_insert(1);
        // }
    }
    let uploaded = uploaded
        .into_iter()
        .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
        .collect::<BTreeMap<_, _>>();
    let uploaders = uploaders
        .into_iter()
        // .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v.len()))
        .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
        .collect::<BTreeMap<_, _>>();
    // let new_uploaders = new_uploaders
    //     .into_iter()
    //     .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
    //     .collect::<BTreeMap<_, _>>();
    // let mut total_size = 0;
    // let size = size
    //     .into_iter()
    //     .map(|(k, v)| {
    //         total_size += v;
    //         (
    //             format!("{:04}-{:02}", k.0, k.1),
    //             total_size / 1024 / 1024 / 1024,
    //         )
    //     })
    //     .collect::<BTreeMap<_, _>>();
    // let uploads = uploads
    //     .into_iter()
    //     .filter(|(_, v)| *v > 5000)
    //     .collect::<BTreeMap<_, _>>();

    // let m4b = m4b
    //     .into_iter()
    //     .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
    //     .collect::<BTreeMap<_, _>>();
    // let mp3 = mp3
    //     .into_iter()
    //     .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
    //     .collect::<BTreeMap<_, _>>();
    // let epub = epub
    //     .into_iter()
    //     .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
    //     .collect::<BTreeMap<_, _>>();
    // let mobi = mobi
    //     .into_iter()
    //     .map(|(k, v)| (format!("{:04}-{:02}", k.0, k.1), v))
    //     .collect::<BTreeMap<_, _>>();

    Ok::<_, AppError>(Json(json!({
        // "torrents": torrents,
        // "uploaded": uploaded,
        // "uploaders": uploaders,
        // "size": size,
        // "categories": categories,
        // "uploads": uploads,
        // "new_uploaders": new_uploaders,
        "goomer": uploaded,
        "fs": uploaders,

        // "total_size": total_size,
        // "m4b": m4b,
        // "mp3": mp3,
        // "epub": epub,
        // "mobi": mobi,
    })))
}
