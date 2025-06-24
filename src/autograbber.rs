use crate::{
    config::{Cost, TorrentFilter, Type},
    mam::{MaM, SearchKind, SearchQuery, SearchTarget, Tor},
};
use anyhow::Result;

pub async fn autograb(
    torrent_filter: &TorrentFilter,
    mam: &MaM<'_>,
    max_torrents: u8,
) -> Result<u8> {
    let target = match torrent_filter.filter.kind {
        Type::Bookmarks => Some(SearchTarget::Bookmarks),
        _ => None,
    };
    let kind = match (torrent_filter.filter.kind, torrent_filter.filter.cost) {
        (Type::Freeleech, _) => Some(SearchKind::Freeleech),
        (_, Cost::Free) => Some(SearchKind::Free),
        _ => None,
    };
    let results = mam
        .search(&SearchQuery {
            dl_link: true,
            perpage: 100.min(max_torrents),
            tor: Tor {
                target,
                kind,
                text: &torrent_filter.filter.query.clone().unwrap_or_default(),
                srch_in: torrent_filter.filter.search_in.clone(),
                main_cat: torrent_filter.filter.categories.get_main_cats(),
                cat: torrent_filter.filter.categories.get_cats(),
                browse_lang: torrent_filter
                    .filter
                    .languages
                    .iter()
                    .map(|l| l.to_id())
                    .collect(),
                max_size: torrent_filter.filter.max_size.bytes(),
                unit: torrent_filter.filter.max_size.unit(),
                ..Default::default()
            },

            ..Default::default()
        })
        .await?;

    println!("{results:#?}");
    Ok(results.found as u8)
}
