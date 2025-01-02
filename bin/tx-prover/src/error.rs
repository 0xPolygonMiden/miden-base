use axum::http::uri::InvalidUri;
use thiserror::Error;

// TX PROVER SERVICE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TxProverServiceError {
    #[error("invalid uri")]
    InvalidURI(#[source] InvalidUri),
    #[error("failed to connect to worker")]
    ConnectionFailed(#[source] tonic::transport::Error),
    #[error("failed to create backend for worker")]
    BackendCreationFailed(#[source] Box<pingora::Error>),
    #[error("failed to setup pingora: {0}")]
    PingoraConfigFailed(String),
}

impl From<TxProverServiceError> for String {
    fn from(err: TxProverServiceError) -> Self {
        err.to_string()
    }
}
