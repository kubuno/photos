use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum PhotosError {
    #[error("Non authentifié")]
    Unauthorized,

    #[error("Accès refusé")]
    Forbidden,

    #[error("Ressource introuvable: {0}")]
    NotFound(String),

    #[error("Données invalides: {0}")]
    Validation(String),

    #[error("Conflit: {0}")]
    Conflict(String),

    #[error("Quota dépassé")]
    QuotaExceeded,

    #[error("Fichier trop volumineux")]
    FileTooLarge,

    #[error("Format non supporté")]
    UnsupportedFormat,

    #[error("Erreur de stockage: {0}")]
    Storage(#[from] kubuno_storage::StorageError),

    #[error("Erreur base de données")]
    Database(#[from] sqlx::Error),

    #[error("Erreur interne")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for PhotosError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            PhotosError::Unauthorized     => (StatusCode::UNAUTHORIZED,           "UNAUTHORIZED",     self.to_string()),
            PhotosError::Forbidden        => (StatusCode::FORBIDDEN,              "FORBIDDEN",        self.to_string()),
            PhotosError::NotFound(_)      => (StatusCode::NOT_FOUND,              "NOT_FOUND",        self.to_string()),
            PhotosError::Validation(_)    => (StatusCode::UNPROCESSABLE_ENTITY,   "VALIDATION",       self.to_string()),
            PhotosError::Conflict(_)      => (StatusCode::CONFLICT,               "CONFLICT",         self.to_string()),
            PhotosError::QuotaExceeded    => (StatusCode::from_u16(507).unwrap(), "QUOTA_EXCEEDED",   self.to_string()),
            PhotosError::FileTooLarge     => (StatusCode::PAYLOAD_TOO_LARGE,      "FILE_TOO_LARGE",   self.to_string()),
            PhotosError::UnsupportedFormat => (StatusCode::UNPROCESSABLE_ENTITY,  "UNSUPPORTED_FORMAT", self.to_string()),
            PhotosError::Storage(e) => {
                tracing::error!(error = %e, "Storage error");
                (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR", "Erreur de stockage".to_string())
            }
            PhotosError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "Erreur base de données".to_string())
            }
            PhotosError::Internal(e) => {
                tracing::error!(error = %e, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Erreur interne".to_string())
            }
        };

        (status, Json(json!({ "error": code, "message": message }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, PhotosError>;
