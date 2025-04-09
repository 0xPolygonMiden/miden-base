use tokio::net::TcpListener;

use crate::{
    commands::worker::ProverTypeSupport,
    generated::{api_server::ApiServer, status::status_api_server::StatusApiServer},
};

mod prover;
mod status;

pub use prover::ProverRpcApi;

pub struct RpcListener {
    pub api_service: ApiServer<ProverRpcApi>,
    pub status_service: StatusApiServer<status::StatusRpcApi>,
    pub listener: TcpListener,
}

impl RpcListener {
    pub fn new(listener: TcpListener, prover_type_support: ProverTypeSupport) -> Self {
        let prover_rpc_api = ProverRpcApi::new(prover_type_support);
        let status_rpc_api = status::StatusRpcApi::new(prover_type_support);
        let api_service = ApiServer::new(prover_rpc_api);
        let status_service = StatusApiServer::new(status_rpc_api);
        Self { listener, api_service, status_service }
    }
}
