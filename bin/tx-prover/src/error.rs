use axum::http::uri::InvalidUri;
use thiserror::Error;

// TX PROVER ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TxProverProxyError {
    #[error("invalid uri error")]
    InvalidURI(#[source] InvalidUri),
    #[error("failed to connect to worker")]
    ConnectionFailed(#[source] tonic::transport::Error),
    #[error("failed to create backend for worker")]
    BackendCreationFailed(#[source] Box<pingora::Error>),
    #[error("app logic not found")]
    AppLogicNotFound,
}

impl From<TxProverProxyError> for String {
    fn from(err: TxProverProxyError) -> Self {
        err.to_string()
    }
}
