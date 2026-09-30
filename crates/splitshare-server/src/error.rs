use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use splitshare_core::StorageError;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}
impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub fn invalid() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "The request is invalid.",
        )
    }
}
#[derive(Serialize)]
struct Envelope {
    error: ErrorBody,
}
#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    details: Option<()>,
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(Envelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                    details: None,
                },
            }),
        )
            .into_response()
    }
}
impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        use StorageError::*;
        let (status, code) = match error {
            InvalidPath => (StatusCode::BAD_REQUEST, "INVALID_PATH"),
            NotFound => (StatusCode::NOT_FOUND, "PATH_NOT_FOUND"),
            Conflict => (StatusCode::CONFLICT, "CONFLICT"),
            RootProtected => (StatusCode::FORBIDDEN, "ROOT_PROTECTED"),
            PermissionDenied => (StatusCode::FORBIDDEN, "PERMISSION_DENIED"),
            UnsupportedEntry => (StatusCode::FORBIDDEN, "UNSUPPORTED_ENTRY"),
            DirectoryNotEmpty => (StatusCode::CONFLICT, "DIRECTORY_NOT_EMPTY"),
            NotFile => (StatusCode::BAD_REQUEST, "NOT_FILE"),
            NotDirectory => (StatusCode::BAD_REQUEST, "NOT_DIRECTORY"),
            UnsupportedOperation => (StatusCode::NOT_IMPLEMENTED, "UNSUPPORTED_OPERATION"),
            LengthMismatch => (StatusCode::BAD_REQUEST, "LENGTH_MISMATCH"),
            UploadFailed => (StatusCode::BAD_REQUEST, "UPLOAD_FAILED"),
            Io => (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR"),
        };
        Self::new(status, code, error.to_string())
    }
}
