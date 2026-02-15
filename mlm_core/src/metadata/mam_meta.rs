use crate::{Context, ContextExt};
use anyhow::Result;
use mlm_db::TorrentMeta;

/// Match metadata for a given original `TorrentMeta` using the selected
/// provider id. This function does NOT persist changes to the database; it
/// performs the provider query and returns the new metadata and the list of
/// diffed fields so the caller can decide how to persist/apply them.
pub async fn match_meta(
    ctx: &Context,
    orig: &TorrentMeta,
    provider_id: &str,
) -> Result<(TorrentMeta, String, Vec<mlm_db::TorrentMetaDiff>)> {
    // Build a small query meta for providers to consume. Providers accept
    // a TorrentMeta and may read any fields they need.
    let mut query: TorrentMeta = Default::default();
    if let Some(isbn) = orig.ids.get(mlm_db::ids::ISBN) {
        query
            .ids
            .insert(mlm_db::ids::ISBN.to_string(), isbn.clone());
    }
    query.title = orig.title.clone();
    query.authors = orig.authors.clone();

    // Delegate provider selection and request-timeout handling to the
    // centralized MetadataService attached to the Context. This keeps
    // provider configuration in one place and avoids duplicating instantiation
    // logic here.
    let fetched = ctx
        .ssr()
        .metadata
        .fetch_provider(ctx, query, provider_id)
        .await?;

    // Merge fetched metadata into original meta: only overwrite fields when
    // the provider supplied non-empty / non-default values. This preserves
    // DB-only fields (sizes, upload timestamps, internal IDs) when providers
    // don't populate them.
    let merged = merge_meta(orig, &fetched);

    let fields = orig.diff(&merged);

    Ok((merged, provider_id.to_string(), fields))
}

fn merge_meta(orig: &TorrentMeta, incoming: &TorrentMeta) -> TorrentMeta {
    let mut out = orig.clone();

    // ids: overlay incoming entries (non-empty) on top of existing ids
    for (k, v) in &incoming.ids {
        if !v.is_empty() {
            out.ids.insert(k.clone(), v.clone());
        }
    }

    if !incoming.title.is_empty() {
        out.title = incoming.title.clone();
    }
    if !incoming.description.is_empty() {
        out.description = incoming.description.clone();
    }

    if !incoming.authors.is_empty() {
        out.authors = incoming.authors.clone();
    }
    if !incoming.narrators.is_empty() {
        out.narrators = incoming.narrators.clone();
    }
    if !incoming.series.is_empty() {
        out.series = incoming.series.clone();
    }

    if !incoming.categories.is_empty() {
        out.categories = incoming.categories.clone();
    }
    if !incoming.tags.is_empty() {
        out.tags = incoming.tags.clone();
    }

    // Simple scalar/option overlays
    if incoming.main_cat.is_some() {
        out.main_cat = incoming.main_cat;
    }
    if incoming.language.is_some() {
        out.language = incoming.language;
    }
    if incoming.flags.is_some() {
        out.flags = incoming.flags;
    }
    if !incoming.filetypes.is_empty() {
        out.filetypes = incoming.filetypes.clone();
    }
    if incoming.num_files != 0 {
        out.num_files = incoming.num_files;
    }
    // size: only overwrite when provider returned a non-zero size
    if incoming.size.bytes() > 0 {
        out.size = incoming.size;
    }
    if incoming.edition.is_some() {
        out.edition = incoming.edition.clone();
    }

    // Always set source to Match for provider-updated data
    out.source = mlm_db::MetadataSource::Match;

    out
}
