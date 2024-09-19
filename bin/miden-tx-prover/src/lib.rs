#[rustfmt::skip]
pub mod domain;
pub mod server;
pub use server::generated::api::{ProveTransactionRequest, ProveTransactionResponse};
