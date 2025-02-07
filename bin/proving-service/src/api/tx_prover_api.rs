use miden_objects::transaction::TransactionWitness;
use miden_tx::{
    utils::{Deserializable, Serializable},
    LocalTransactionProver, TransactionProver,
};
use tokio::{net::TcpListener, sync::Mutex};
use tonic::{Request, Response, Status};
use tracing::instrument;

use super::internal_error;
use crate::{
    api::invalid_argument,
    generated::tx_prover::{
        api_server::{Api as TxProverApi, ApiServer as TxApiServer},
        ProveTransactionRequest, ProveTransactionResponse,
    },
    utils::MIDEN_PROVING_SERVICE,
};

// PROVER RPC LISTENER
// ================================================================================================

pub struct TxProverRpcListener {
    pub api_service: TxApiServer<TxProverRpcApi>,
    pub listener: TcpListener,
}

impl TxProverRpcListener {
    pub fn new(listener: TcpListener) -> Self {
        let api_service = TxApiServer::new(TxProverRpcApi::default());
        Self { listener, api_service }
    }
}

#[derive(Default)]
pub struct TxProverRpcApi {
    local_prover: Mutex<LocalTransactionProver>,
}

#[async_trait::async_trait]
impl TxProverApi for TxProverRpcApi {
    #[instrument(
        target = MIDEN_PROVING_SERVICE,
        name = "prover:prove_transaction",
        skip_all,
        ret(level = "debug"),
        fields(transaction_id = tracing::field::Empty),
        err
    )]
    async fn prove_transaction(
        &self,
        request: Request<ProveTransactionRequest>,
    ) -> Result<Response<ProveTransactionResponse>, tonic::Status> {
        // Try to acquire a permit without waiting
        let prover = self
            .local_prover
            .try_lock()
            .map_err(|_| Status::resource_exhausted("Server is busy handling another request"))?;

        let transaction_witness =
            TransactionWitness::read_from_bytes(&request.get_ref().transaction_witness)
                .map_err(invalid_argument)?;

        let proof = prover.prove(transaction_witness).map_err(internal_error)?;

        // Record the transaction_id in the current tracing span
        let transaction_id = proof.id();
        tracing::Span::current().record("transaction_id", tracing::field::display(&transaction_id));

        Ok(Response::new(ProveTransactionResponse { proven_transaction: proof.to_bytes() }))
    }
}
