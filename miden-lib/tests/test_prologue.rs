pub mod common;
use common::{
    consumed_note_data_ptr,
    data::{mock_inputs, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, NONCE},
    memory::{
        ACCT_CODE_ROOT_PTR, ACCT_ID_AND_NONCE_PTR, ACCT_ID_PTR, ACCT_STORAGE_ROOT_PTR,
        ACCT_VAULT_ROOT_PTR, BATCH_ROOT_PTR, BLK_HASH_PTR, BLOCK_NUM_PTR, CHAIN_MMR_NUM_LEAVES_PTR,
        CHAIN_MMR_PEAKS_PTR, CHAIN_ROOT_PTR, CONSUMED_NOTE_SECTION_OFFSET, INIT_ACCT_HASH_PTR,
        NOTE_ROOT_PTR, NULLIFIER_COM_PTR, PREV_BLOCK_HASH_PTR, PROOF_HASH_PTR, STATE_ROOT_PTR,
    },
    run_within_tx_kernel, AdviceProvider, Felt, FieldElement, MemAdviceProvider, Process,
    TransactionInputs, Word, TX_KERNEL_DIR,
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
        Some(TX_KERNEL_DIR),
        Some(PROLOGUE_FILE),
    )
    .unwrap();

    public_input_memory_assertions(&process, &inputs);
    block_data_memory_assertions(&process, &inputs);
    chain_mmr_memory_assertions(&process, &inputs);
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
        inputs.block_header().hash().as_elements()
    );

    // The account ID should be stored at the ACCT_ID_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_ID_PTR).unwrap(),
        [inputs.account().id().into(), Felt::ZERO, Felt::ZERO, inputs.account().nonce()]
    );

    // The account commitment should be stored at the ACCT_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, INIT_ACCT_HASH_PTR).unwrap(),
        inputs.account().hash().as_elements()
    );

    // The nullifier commitment should be stored at the NULLIFIER_COM_PTR
    assert_eq!(
        process.get_memory_value(0, NULLIFIER_COM_PTR).unwrap(),
        inputs.consumed_notes_commitment().as_elements()
    );
}

fn block_data_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The block hash should be stored at the BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, BLK_HASH_PTR).unwrap(),
        inputs.block_header().hash().as_elements()
    );

    // The previous block hash should be stored at the PREV_BLK_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, PREV_BLOCK_HASH_PTR).unwrap(),
        inputs.block_header().prev_hash().as_elements()
    );

    // The chain root should be stored at the CHAIN_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, CHAIN_ROOT_PTR).unwrap(),
        inputs.block_header().chain_root().as_elements()
    );

    // The state root should be stored at the STATE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, STATE_ROOT_PTR).unwrap(),
        inputs.block_header().state_root().as_elements()
    );

    // The batch root should be stored at the BATCH_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, BATCH_ROOT_PTR).unwrap(),
        inputs.block_header().batch_root().as_elements()
    );

    // The note root should be stored at the NOTE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, NOTE_ROOT_PTR).unwrap(),
        inputs.block_header().note_root().as_elements()
    );

    // The proof hash should be stored at the PROOF_HASH_PTR
    assert_eq!(
        process.get_memory_value(0, PROOF_HASH_PTR).unwrap(),
        inputs.block_header().proof_hash().as_elements()
    );

    // The block number should be stored at the BLOCK_NUM_PTR
    assert_eq!(
        process.get_memory_value(0, BLOCK_NUM_PTR).unwrap()[0],
        inputs.block_header().block_num()
    );
}

fn chain_mmr_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The number of leaves should be stored at the CHAIN_MMR_NUM_LEAVES_PTR
    assert_eq!(
        process.get_memory_value(0, CHAIN_MMR_NUM_LEAVES_PTR).unwrap()[0],
        Felt::new(inputs.block_chain().mmr().forest() as u64)
    );

    for (i, peak) in inputs.block_chain().mmr().accumulator().peaks.iter().enumerate() {
        // The peaks should be stored at the CHAIN_MMR_PEAKS_PTR
        assert_eq!(&process.get_memory_value(0, CHAIN_MMR_PEAKS_PTR + i as u64).unwrap(), peak);
    }
}

fn account_data_memory_assertions<A: AdviceProvider>(
    process: &Process<A>,
    inputs: &TransactionInputs,
) {
    // The account id should be stored at ACCT_ID_AND_NONCE_PTR[0]
    assert_eq!(
        process.get_memory_value(0, ACCT_ID_AND_NONCE_PTR).unwrap()[0],
        Felt::new(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN)
    );

    // The account nonce should be stored at ACCT_ID_AND_NONCE_PTR[3]
    assert_eq!(process.get_memory_value(0, ACCT_ID_AND_NONCE_PTR).unwrap()[3], NONCE);

    // The account vault root commitment should be stored at ACCT_VAULT_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_VAULT_ROOT_PTR).unwrap(),
        inputs.account().vault().commitment().as_elements()
    );

    // The account storage root commitment should be stored at ACCT_STORAGE_ROOT_PTR
    assert_eq!(
        process.get_memory_value(0, ACCT_STORAGE_ROOT_PTR).unwrap(),
        inputs.account().storage().root()
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
        process.get_memory_value(0, CONSUMED_NOTE_SECTION_OFFSET).unwrap()[0],
        Felt::new(inputs.consumed_notes().len() as u64)
    );

    for (note, note_idx) in inputs.consumed_notes().iter().zip(0u64..) {
        // The note nullifier should be computer and stored at (CONSUMED_NOTES_OFFSET + 1 + note_idx)
        assert_eq!(
            process
                .get_memory_value(0, CONSUMED_NOTE_SECTION_OFFSET + 1 + note_idx)
                .unwrap(),
            note.nullifier().as_elements()
        );

        // The note hash should be computed and stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx)).unwrap(),
            note.hash().as_elements()
        );

        // The note serial num should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 1)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 1).unwrap(),
            note.serial_num()
        );

        // The note script hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 2)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 2).unwrap(),
            note.script().hash().as_elements()
        );

        // The note input hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 3)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 3).unwrap(),
            note.inputs().hash().as_elements()
        );

        // The note vault hash should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 4)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 4).unwrap(),
            note.vault().hash().as_elements()
        );

        // The number of assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 5)
        assert_eq!(
            process.get_memory_value(0, consumed_note_data_ptr(note_idx) + 5).unwrap(),
            Word::from(note.metadata())
        );

        // The assets should be stored at (CONSUMED_NOTES_OFFSET + (note_index + 1) * 1024 + 6..)
        for (asset, asset_idx) in note.vault().iter().cloned().zip(0u64..) {
            let word: Word = asset.into();
            assert_eq!(
                process
                    .get_memory_value(0, consumed_note_data_ptr(note_idx) + 6 + asset_idx)
                    .unwrap(),
                word
            );
        }
    }
}
