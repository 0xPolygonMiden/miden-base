use std::sync::Arc;

use axum::{
    extract::State, http::StatusCode, routing::{get, post}, Json, Router
};
use miden_lib::transaction;
use miden_objects::transaction::TransactionWitness;
use miden_tx::{utils::Deserializable, LocalTransactionProver, TransactionProver};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    // Initialize the common prover
    let prover = Arc::new(LocalTransactionProver::default());

    // build our application with a route
    let app = Router::new().route("/prove", post(prove_transaction)).with_state(prover);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn prove_transaction(
    prove_transaction_request: ProveTransactionRequest,
    State(prover): State<Arc<LocalTransactionProver>>,
) -> (StatusCode, ProveTransactionResponse) {
    let transaction_witness = TransactionWitness::read_from_bytes(&prove_transaction_request.transaction_witness).unwrap();
    let transaction_prove = prover
        .prove(transaction_witness)
        .unwrap();
    (StatusCode::OK, transaction_prove.into())
}

struct ProveTransactionRequest {
    transaction_witness: Vec<u8>,
}

struct ProveTransactionResponse {
    proven_transaction: Vec<u8>
}
