use core::fmt;
use std::{
    fs::{read_to_string, write, File},
    io::Write,
    path::Path,
};

use miden_lib::{notes::create_p2id_note, transaction::TransactionKernel};
use miden_objects::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    transaction::{TransactionArgs, TransactionMeasurements, TransactionScript},
    Felt,
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};
use vm_processor::ONE;

mod utils;
use utils::{
    get_account_with_default_account_code, get_new_pk_and_authenticator,
    write_bench_results_to_json, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER, DEFAULT_AUTH_SCRIPT,
};
pub enum Benchmark {
    Simple,
    P2ID,
}

impl fmt::Display for Benchmark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Benchmark::Simple => write!(f, "simple"),
            Benchmark::P2ID => write!(f, "p2id"),
        }
    }
}

fn main() -> Result<(), String> {
    // create a template file for benchmark results
    let path = Path::new("bench-tx/bench-tx.json");
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(b"{}").map_err(|e| e.to_string())?;

    // run all available benchmarks
    let benchmark_results = vec![
        // (Benchmark::Simple, benchmark_default_tx()?),
        (Benchmark::P2ID, benchmark_p2id()?),
    ];

    // store benchmark results in the JSON file
    write_bench_results_to_json(path, benchmark_results)?;

    Ok(())
}

// BENCHMARKS
// ================================================================================================

/*
/// Runs the default transaction with empty transaction script and two default notes.
pub fn benchmark_default_tx() -> Result<TransactionMeasurements, String> {
    let tx_context = TransactionContextBuilder::with_standard_account(ONE)
        .with_mock_notes_preserved()
        .build();

    // std::println!("{:?}", tx_context);

    // std::println!("initial: {:?}\n", tx_context.tx_inputs().account().vault());

    let executor: TransactionExecutor<_, ()> = TransactionExecutor::new(tx_context.clone(), None)
        .with_tracing()
        .with_debug_mode(true);

    let account_id = tx_context.account().id();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| {
            // std::println!("\nnote: {:?}\n", note.note().assets());
            note.id()
        })
        .collect::<Vec<_>>();

    // std::println!("\n{:?}", note_ids);

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, tx_context.tx_args().clone())
        .map_err(|e| e.to_string())?;

    Ok(executed_transaction.into())
}
*/

/// Runs the transaction which consumes a P2ID note into a basic wallet.
pub fn benchmark_p2id() -> Result<TransactionMeasurements, String> {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let (target_pub_key, falcon_auth) = get_new_pk_and_authenticator();

    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let tx_context = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note.clone()])
        .build();

    let executor =
        TransactionExecutor::new(tx_context.clone(), Some(falcon_auth.clone())).with_tracing();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let tx_script_target =
        TransactionScript::compile(DEFAULT_AUTH_SCRIPT, [], TransactionKernel::assembler())
            .unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // execute transaction
    let executed_transaction = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .unwrap();

    Ok(executed_transaction.into())
}
