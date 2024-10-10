extern crate alloc;

/// Contains the protobuf definitions
pub const PROTO_MESSAGES: &str = include_str!("../proto/api.proto");

/// Writes the RPC protobuf file into `target_dir`.
#[cfg(feature = "std")]
pub fn write_proto(target_dir: &std::path::Path) -> Result<(), std::string::String> {
    use std::{
        format,
        fs::{self, File},
        string::ToString,
    };

    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .map_err(|err| format!("Error creating directory: {}", err))?;
    } else if !target_dir.is_dir() {
        return Err("The target path exists but is not a directory".to_string());
    }

    let mut file = File::create(&target_dir.join("api.proto"))
        .map_err(|err| format!("Error creating api.rs: {}", err))?;

    file.write_all(PROTO_MESSAGES.as_bytes());

    Ok(())
}

// REMOTE TRANSACTION PROVER
// ================================================================================================

// /// A [RemoteTransactionProver] is a transaction prover that sends witness data to a remote
// /// gRPC server and receives a proven transaction.
// #[derive(Clone)]
// pub struct RemoteTransactionProver {
//     client: RefCell<ProverApiClient<Client>>, // Use RefCell to allow interior mutability.
// }

// impl RemoteTransactionProver {
//     /// Creates a new [RemoteTransactionProver] with the specified gRPC server endpoint.
//     /// This instantiates a tonic client that attempts connecting with the server.
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if the endpoint is invalid or if the gRPC
//     /// connection to the server cannot be established.
//     pub async fn new(endpoint: &str) -> Result<Self, RemoteTransactionProverError> {
//         let client = ProverApiClient::new(Client::new(endpoint.to_string()));

//         Ok(RemoteTransactionProver { client: RefCell::new(client) })
//     }
// }

// #[async_trait(?Send)]
// impl TransactionProver for RemoteTransactionProver {
//     fn prove(
//         &self,
//         tx_witness: TransactionWitness,
//     ) -> Result<ProvenTransaction, TransactionProverError> {
//         let mut client = self.client.borrow_mut();

//         let request = tonic::Request::new(ProveTransactionRequest {
//             transaction_witness: tx_witness.to_bytes(),
//         });

//         let response = client
//             .prove_transaction(request)
//             .await
//             .map_err(|err| TransactionProverError::InternalError(err.to_string()))?;

//         // Deserialize the response bytes back into a ProvenTransaction.
//         let proven_transaction = ProvenTransaction::try_from(response.into_inner())
//             .map_err(TransactionProverError::DeserializationError)?;

//         Ok(proven_transaction)
//     }
// }

// #[derive(Debug)]
// pub enum RemoteTransactionProverError {
//     /// Indicates that the provided gRPC server endpoint is invalid.
//     InvalidEndpoint(String),

//     /// Indicates that the connection to the server failed.
//     ConnectionFailed(String, tonic::transport::Error),
// }
