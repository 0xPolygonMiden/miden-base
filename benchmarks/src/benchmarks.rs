use miden_lib::notes::create_p2id_note;
use miden_lib::transaction::ToTransactionKernelInputs;
use miden_lib::utils::Serializable;
use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::{Asset, FungibleAsset},
    crypto::dsa::rpo_falcon512::SecretKey,
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    transaction::TransactionArgs,
    Felt,
};
use miden_tx::{TransactionExecutor, TransactionHost};
use vm_processor::{ExecutionOptions, RecAdviceProvider, Word};

use crate::{
    utils::{
        get_account_with_default_account_code, write_cycles_to_json, MockDataStore, String,
        ToString, Vec, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN, ACCOUNT_ID_SENDER,
        DEFAULT_AUTH_SCRIPT,
    },
    Path,
};

// BENCHMARKS
// ================================================================================================

/// Runs the default transaction with empty transaction script and two default notes.
pub fn benchmark_default_tx(path: &Path) -> Result<(), String> {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone()).with_tracing();

    let account_id = data_store.account.id();
    executor.load_account(account_id).map_err(|e| e.to_string())?;

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let transaction = executor
        .prepare_transaction(account_id, block_ref, &note_ids, data_store.tx_args().clone())
        .map_err(|e| e.to_string())?;

    let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
    let advice_recorder: RecAdviceProvider = advice_inputs.into();
    let mut host = TransactionHost::new(transaction.account().into(), advice_recorder);

    vm_processor::execute(
        transaction.program(),
        stack_inputs,
        &mut host,
        ExecutionOptions::default().with_tracing(),
    )
    .map_err(|e| e.to_string())?;

    #[cfg(feature = "std")]
    write_cycles_to_json(path, crate::Benchmark::Simple, host.tx_progress())?;

    Ok(())
}

/// Runs the transaction which consumes a P2ID note into a basic wallet.
pub fn benchmark_p2id(path: &Path) -> Result<(), String> {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN).unwrap();
    let sec_key = SecretKey::new();
    let target_pub_key: Word = sec_key.public_key().into();
    let mut pk_sk_bytes = sec_key.to_bytes();
    pk_sk_bytes.append(&mut target_pub_key.to_bytes());
    let target_sk_pk_felt: Vec<Felt> =
        pk_sk_bytes.iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>();
    let target_account =
        get_account_with_default_account_code(target_account_id, target_pub_key, None);

    // Create the note
    let note = create_p2id_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone()).with_tracing();
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(DEFAULT_AUTH_SCRIPT).unwrap();

    let tx_script_target = executor
        .compile_tx_script(
            tx_script_code.clone(),
            vec![(target_pub_key, target_sk_pk_felt)],
            vec![],
        )
        .unwrap();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // execute transaction
    let transaction = executor
        .prepare_transaction(target_account_id, block_ref, &note_ids, tx_args_target)
        .map_err(|e| e.to_string())?;

    let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
    let advice_recorder: RecAdviceProvider = advice_inputs.into();
    let mut host = TransactionHost::new(transaction.account().into(), advice_recorder);

    vm_processor::execute(
        transaction.program(),
        stack_inputs,
        &mut host,
        ExecutionOptions::default().with_tracing(),
    )
    .map_err(|e| e.to_string())?;

    #[cfg(feature = "std")]
    write_cycles_to_json(path, crate::Benchmark::P2ID, host.tx_progress())?;

    Ok(())
}

/// Runs all available benchmarks.
pub fn benchmark_all(path: &Path) -> Result<(), String> {
    benchmark_default_tx(path)?;
    benchmark_p2id(path)
}
