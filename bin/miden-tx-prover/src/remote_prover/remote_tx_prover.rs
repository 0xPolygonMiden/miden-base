use std::sync::Arc;
use miden_objects::transaction::{ProvenTransaction, TransactionWitness};
use miden_tx::{
    utils::{Deserializable, Serializable},
    TransactionProver, TransactionProverError,
};
use tokio::sync::Mutex;

use crate::{ProveTransactionRequest, ProveTransactionResponse};
use crate::server::generated::api::api_client::ApiClient;
use tonic::transport::Channel;
use tokio::time::Duration;

#[derive(Debug)]
enum RpcError {
    ConnectionError(String),
}

#[derive(Clone)]
pub struct RemoteTransactionProver {
    rpc_api: Option<ApiClient<Channel>>,
    endpoint: String,
    timeout_ms: u64,
}

impl RemoteTransactionProver {
    pub fn new(endpoint: String, timeout_ms: u64) -> Self {
        Self {
            rpc_api: None,
            endpoint,
            timeout_ms,
        }
    }

    async fn rpc_api(&mut self) -> Result<&mut ApiClient<Channel>, RpcError> {
        if self.rpc_api.is_some() {
            Ok(self.rpc_api.as_mut().unwrap())
        } else {
            let endpoint = tonic::transport::Endpoint::try_from(self.endpoint.clone())
                .map_err(|err| RpcError::ConnectionError(err.to_string()))?
                .timeout(Duration::from_millis(self.timeout_ms));
            let rpc_api = ApiClient::connect(endpoint)
                .await
                .map_err(|err| RpcError::ConnectionError(err.to_string()))?;
            Ok(self.rpc_api.insert(rpc_api))
        }
    }

    pub(crate) async fn prove(
        &mut self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        let tx_witness: TransactionWitness = transaction.into();

        let tx_witness_request = ProveTransactionRequest {
            transaction_witness: tx_witness.to_bytes(),
        };

        let rpc_api = self.rpc_api().await.unwrap();
        let proven_transaction: ProvenTransaction = rpc_api.prove_transaction(tx_witness_request).await.unwrap().into_inner().try_into().unwrap();

        Ok(proven_transaction)
    }
}

impl TransactionProver for RemoteTransactionProver {
    async fn prove(
        &self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        self.prove(transaction).await
    }
}
