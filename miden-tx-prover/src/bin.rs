use axum::{
    http::StatusCode,
    routing::post,
    Router,
};
use miden_objects::transaction::TransactionWitness;
use miden_tx::{utils::Deserializable, LocalTransactionProver, TransactionProver};
use miden_tx_prover::{ProveTransactionRequest, ProveTransactionResponse};
use winter_maybe_async::maybe_await;

#[tokio::main]
async fn main() {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new().route("/prove", post(prove_transaction));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn prove_transaction(
    prove_transaction_request: ProveTransactionRequest,
) -> (StatusCode, ProveTransactionResponse) {
    let prover = LocalTransactionProver::default();

    let transaction_witness = TransactionWitness::read_from_bytes(
        &prove_transaction_request.transaction_witness
        ).unwrap();

    let transaction_prove = maybe_await!(prover.prove(transaction_witness)).unwrap();
    (StatusCode::OK, transaction_prove.into())
}
