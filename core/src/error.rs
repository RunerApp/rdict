use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("malformed input: {0}")]
    Malformed(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("zstd error: {0}")]
    Zstd(String),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(String),

    #[error("not found: {0}")]
    NotFound(String),
}
