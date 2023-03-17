mod common;
use common::{
    memory::*, mock_inputs, note_data_ptr, run_within_tx_kernel, AdviceProvider, Felt,
    FieldElement, MemAdviceProvider, Process, TransactionInputs, Word,
    ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN, NONCE, TX_KERNEL_DIR,
};

const PROLOGUE_FILE: &str = "prologue.masm";

#[test]
fn test_transaction_prologue() {
    let inputs = mock_inputs();
    let code = "
        begin
            exec.prepare_transaction
        end
        ";
    let process = run_within_tx_kernel(
        "",
        code,
        inputs.stack_inputs(),
        MemAdviceProvider::from(inputs.advice_provider_inputs()),
        TX_KERNEL_DIR,
        PROLOGUE_FILE,
    );

    public_input_memory_assertions(&process, &inputs);
    account_data_memory_assertions(&process, &inputs);
    consumed_notes_memory_assertions(&process, &inputs);
}

fn public_input_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, BLK_HASH_PTR).unwrap(),
        inputs.block_ref().as_elements()
    );

    // The account ID should be stored at the ACCT_ID_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_ID_PTR).unwrap(),
        [inputs.account().id().into(), Felt::ZERO, Felt::ZERO, inputs.account().nonce()]
    );

    // The account commitment should be stored at the ACCT_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_HASH_PTR).unwrap(),
        inputs.account().hash().as_elements()
    );

    // The nullifier commitment should be stored at the NULLIFIER_COM_PTR
    assert_eq!(
        process.get_memory_value(0, NULLIFIER_COM_PTR).unwrap(),
        inputs.consumed_notes_nullifier_commitment().as_elements()
    );
}

fn account_data_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The account id should be stored at ACCT_NONCE_PTR[0]
    assert_eq!(
        process.get_memory_value(0, ACCT_NONCE_PTR).unwrap()[0],
        Felt::new(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
    );

    // The account nonce should be stored at ACCT_NONCE_PTR[3]
    assert_eq!(process.get_memory_value(0, ACCT_NONCE_PTR).unwrap()[3], NONCE);

    // The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_VAULT_ROOT_PTR).unwrap(),
        inputs.account().vault().root().as_elements()
    );

    // The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_STORAGE_ROOT_PTR).unwrap(),
        inputs.account().storage().root().as_elements()
    );

    // The account code commitment should be stored at (ACCOUNT_DATA_OFFSET + 4)
    assert_eq!(
        process.get_memory_value(0, ACCT_CODE_ROOT_PTR).unwrap(),
        inputs.account().code().root().as_elements()
    );
}

fn consumed_notes_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The number of consumed notes should be stored at the CONSUMED_NOTES_OFFSET
    assert_eq!(
        process.get_memory_value(0, CONSUMED_NOTE_DATA_OFFSET).unwrap()[0],
        Felt::new(inputs.consumed_notes().len() as u64)
    );

    for (note, note_idx) in inputs.consumed_notes().iter().zip(0u64..) {
        // The note nullifier should be computer and stored at (CONSUMED_NOTES_OFFSET + 1 + note_idx)
        assert_eq!(
            process.get_memory_value(0, CONSUMED_NOTE_DATA_OFFSET + 1 + note_idx).unwrap(),
            note.get_nullifier().as_elements()
        );

        // The note hash should be computed and stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx)).unwrap(),
            note.get_hash().as_elements()
        );

        // The note serial num should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 1)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx) + 1).unwrap(),
            note.serial_num()
        );

        // The note script hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 2)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx) + 2).unwrap(),
            note.script().hash().as_elements()
        );

        // The note input hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 3)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx) + 3).unwrap(),
            note.inputs().hash().as_elements()
        );

        // The note vault hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 4)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx) + 4).unwrap(),
            note.vault().hash().as_elements()
        );

        // The number of assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 5)
        assert_eq!(
            process.get_memory_value(0, note_data_ptr(note_idx) + 5).unwrap(),
            [Felt::new(note.vault().num_assets() as u64), Felt::ZERO, Felt::ZERO, Felt::ZERO]
        );

        // The assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 6..)
        for (asset, asset_idx) in note.vault().iter().cloned().zip(0u64..) {
            let word: Word = asset.into();
            assert_eq!(
                process.get_memory_value(0, note_data_ptr(note_idx) + 6 + asset_idx).unwrap(),
                word
            );
        }
    }
}
