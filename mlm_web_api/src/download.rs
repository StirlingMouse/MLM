use axum::{
    body::Body,
    extract::{Path, State},
    response::IntoResponse,
};
use mlm_db::Torrent;
use tokio_util::io::ReaderStream;
use tracing::warn;

use crate::error::AppError;
use mlm_core::{
    Context, ContextExt,
    linker::map_path,
    qbittorrent::{self},
};

pub async fn torrent_file(
    State(context): State<Context>,
    Path((id, filename)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let config = context.config().await;
    let Some(torrent) = context.db().r_transaction()?.get().primary::<Torrent>(id)? else {
        return Err(AppError::NotFound);
    };
    let Some(path) = (if let (Some(library_path), Some(library_file)) = (
        &torrent.library_path,
        torrent
            .library_files
            .iter()
            .find(|f| f.to_string_lossy() == filename),
    ) {
        Some(library_path.join(library_file))
    } else if let Some((torrent, qbit, qbit_config)) =
        qbittorrent::get_torrent(&config, &torrent.id).await?
    {
        qbit.files(&torrent.hash, None)
            .await?
            .into_iter()
            .find(|f| f.name == filename)
            .map(|file| map_path(&qbit_config.path_mapping, &torrent.save_path).join(&file.name))
    } else {
        None
    }) else {
        return Err(AppError::NotFound);
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) => {
            warn!("Failed opening torrent file {}: {err}", path.display());
            return Err(AppError::NotFound);
        }
    };
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let content_type = mime_guess::from_path(&filename)
        .first_or_octet_stream()
        .to_string();
    let safe_filename = filename.replace(['\r', '\n', '"'], "_");

    let headers = [
        (axum::http::header::CONTENT_TYPE, content_type),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", safe_filename),
        ),
    ];

    Ok((headers, body))
}
