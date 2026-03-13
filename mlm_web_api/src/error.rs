use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Could not query db: {0}")]
    Db(#[from] native_db::db_type::Error),
    #[error("Meta Error: {0:?}")]
    MetaError(#[from] mlm_mam::meta::MetaError),
    #[error("Qbit Error: {0:?}")]
    QbitError(#[from] qbit::Error),
    #[error("Toml Parse Error: {0:?}")]
    Toml(#[from] toml::de::Error),
    #[error("Error: {0:?}")]
    Generic(#[from] anyhow::Error),
    #[error("JSON Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Page Not Found")]
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
