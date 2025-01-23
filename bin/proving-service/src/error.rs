use axum::http::uri::InvalidUri;
use thiserror::Error;

// TX PROVER SERVICE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TxProverServiceError {
    #[error("invalid uri {1}")]
    InvalidURI(#[source] InvalidUri, String),
    #[error("failed to connect to worker {1}")]
    ConnectionFailed(#[source] tonic::transport::Error, String),
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
