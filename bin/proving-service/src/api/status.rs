use tonic::{Request, Response, Status};

use crate::{
    commands::worker::ProverTypeSupport,
    generated::status::{
        StatusRequest, StatusResponse, SupportedProofTypes, status_api_server::StatusApi,
    },
};

pub struct StatusRpcApi {
    prover_type_support: ProverTypeSupport,
}

impl StatusRpcApi {
    pub fn new(prover_type_support: ProverTypeSupport) -> Self {
        Self { prover_type_support }
    }
}

#[async_trait::async_trait]
impl StatusApi for StatusRpcApi {
    async fn status(&self, _: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
        let supported_proof_types = SupportedProofTypes {
            transaction: self.prover_type_support.supports_transaction(),
            batch: self.prover_type_support.supports_batch(),
            block: self.prover_type_support.supports_block(),
        };

        Ok(Response::new(StatusResponse {
            ready: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            supported_proof_types: Some(supported_proof_types),
        }))
    }
}
