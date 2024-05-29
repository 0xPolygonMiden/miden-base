use alloc::vec::Vec;

use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        },
        testing::{
            prepare_word,
            transaction::{mock_inputs, notes::AssetPreservationStatus},
            MockAccountType,
        },
    },
    notes::{Note, NoteAssets, NoteInputs, NoteMetadata, NoteRecipient, NoteType},
    testing::non_fungible_asset,
    transaction::{OutputNote, OutputNotes},
};
use miden_tx::host::testing::{
    utils::{prepare_transaction, run_tx, run_within_tx_kernel},
    MockHost,
};
use vm_processor::Process;

use super::{ContextId, Felt, MemAdviceProvider, ProcessState, StackInputs, Word, ONE, ZERO};
use crate::transaction::{
    memory::{
        CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET,
        CREATED_NOTE_SECTION_OFFSET, NOTE_MEM_SIZE, NUM_CREATED_NOTES_PTR,
    },
    TransactionKernel,
};
#[test]
fn test_create_note() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );
    let account_id = tx_inputs.account().id();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        process.get_mem_value(ContextId::root(), NUM_CREATED_NOTES_PTR).unwrap(),
        [ONE, ZERO, ZERO, ZERO],
        "number of created notes must increment by 1",
    );

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_RECIPIENT_OFFSET),
        recipient,
        "recipient must be stored at the correct memory location",
    );

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_METADATA_OFFSET),
        [tag, Felt::from(account_id), NoteType::Public.into(), ZERO],
        "metadata must be stored at the correct memory location",
    );

    let note_ptr = CREATED_NOTE_SECTION_OFFSET;
    assert_eq!(
        process.stack.get(0),
        Felt::from(note_ptr),
        "top item on the stack is a pointer to the created note"
    );
}

#[test]
fn test_create_note_with_invalid_tag() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new((NoteType::Public as u64) << 62);

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction);

    assert!(process.is_err(), "Transaction should have failed because the tag is invalid");
}

#[test]
fn test_create_note_too_many_notes() {
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let code = format!(
        "
    use.miden::kernels::tx::constants
    use.miden::kernels::tx::memory
    use.miden::tx

    begin
        exec.constants::get_max_num_created_notes
        exec.memory::set_num_created_notes

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        tag = tag,
        PUBLIC_NOTE = NoteType::Public as u8,
    );

    let process =
        run_within_tx_kernel("", &code, StackInputs::default(), MemAdviceProvider::default(), None);

    // assert the process failed
    assert!(process.is_err());
}

#[test]
fn test_get_output_notes_hash() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );

    // extract input note data
    let input_note_1 = tx_inputs.input_notes().get_note(0).note();
    let input_asset_1 = **input_note_1.assets().iter().take(1).collect::<Vec<_>>().first().unwrap();
    let input_note_2 = tx_inputs.input_notes().get_note(1).note();
    let input_asset_2 = **input_note_2.assets().iter().take(1).collect::<Vec<_>>().first().unwrap();

    // create output note 1
    let output_serial_no_1 = [Felt::new(8); 4];
    let output_tag_1 = 8888.into();
    let assets = NoteAssets::new(vec![input_asset_1]).unwrap();
    let metadata =
        NoteMetadata::new(tx_inputs.account().id(), NoteType::Public, output_tag_1, ZERO).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(output_serial_no_1, input_note_1.script().clone(), inputs);
    let output_note_1 = Note::new(assets, metadata, recipient);

    // create output note 2
    let output_serial_no_2 = [Felt::new(11); 4];
    let output_tag_2 = 1111.into();
    let assets = NoteAssets::new(vec![input_asset_2]).unwrap();
    let metadata =
        NoteMetadata::new(tx_inputs.account().id(), NoteType::Public, output_tag_2, ZERO).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(output_serial_no_2, input_note_2.script().clone(), inputs);
    let output_note_2 = Note::new(assets, metadata, recipient);

    // compute expected output notes hash
    let expected_output_notes_hash = OutputNotes::new(vec![
        OutputNote::Full(output_note_1.clone()),
        OutputNote::Full(output_note_2.clone()),
    ])
    .unwrap()
    .commitment();

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        # => [BH, acct_id, IAH, NC]
        exec.prologue::prepare_transaction
        # => []

        # create output note 1
        push.{recipient_1}
        push.{PUBLIC_NOTE}
        push.{tag_1}
        exec.tx::create_note
        # => [note_ptr]

        push.{asset_1} movup.4
        exec.tx::add_asset_to_note


        drop
        # => []

        # create output note 2
        push.{recipient_2}
        push.{PUBLIC_NOTE}
        push.{tag_2}
        exec.tx::create_note
        # => [note_ptr]

        push.{asset_2} movup.4
        exec.tx::add_asset_to_note

        drop
        # => []

        # compute the output notes hash
        exec.tx::get_output_notes_hash
        # => [COMM]
    end
    ",
        PUBLIC_NOTE = NoteType::Public as u8,
        recipient_1 = prepare_word(&output_note_1.recipient().digest()),
        tag_1 = output_note_1.metadata().tag(),
        asset_1 = prepare_word(&Word::from(
            **output_note_1.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
        recipient_2 = prepare_word(&output_note_2.recipient().digest()),
        tag_2 = output_note_2.metadata().tag(),
        asset_2 = prepare_word(&Word::from(
            **output_note_2.assets().iter().take(1).collect::<Vec<_>>().first().unwrap()
        )),
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        process.get_mem_value(ContextId::root(), NUM_CREATED_NOTES_PTR),
        Some([Felt::new(2), ZERO, ZERO, ZERO]),
        "The test creates two notes",
    );
    assert_eq!(
        process.get_mem_value(
            ContextId::root(),
            CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_METADATA_OFFSET
        ),
        Some(output_note_1.metadata().into()),
        "Validate the output note 1 metadata",
    );
    assert_eq!(
        process.get_mem_value(
            ContextId::root(),
            CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_METADATA_OFFSET + NOTE_MEM_SIZE
        ),
        Some(output_note_2.metadata().into()),
        "Validate the output note 1 metadata",
    );

    assert_eq!(process.get_stack_word(0), *expected_output_notes_hash);
}

#[test]
fn test_create_note_and_add_asset() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx
    use.miden::kernels::tx::memory

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}

        exec.tx::create_note
        # => [note_ptr]

        push.{asset} movup.4
        exec.tx::add_asset_to_note

    end
    ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        asset = prepare_word(&asset),
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    let note_ptr = CREATED_NOTE_SECTION_OFFSET;
    assert_eq!(
        process.stack.get(0),
        Felt::from(note_ptr),
        "top item on the stack is a pointer to the created note"
    );
}

#[test]
fn test_create_note_and_add_multiple_assets() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];
    let asset_2 = [Felt::new(20), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let asset_3 = [Felt::new(30), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let asset_2_and_3 =
        [Felt::new(50), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2)];
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let non_fungible_asset_encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}

        exec.tx::create_note
        # => [note_ptr]

        push.{asset} movup.4
        exec.tx::add_asset_to_note
        # => [note_ptr]

        push.{asset_2} movup.4
        exec.tx::add_asset_to_note
        # => [note_ptr]

        push.{asset_3} movup.4
        exec.tx::add_asset_to_note
        # => [note_ptr]

        push.{nft} movup.4
        exec.tx::add_asset_to_note
        # => [note_ptr]
    end
    ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        asset = prepare_word(&asset),
        asset_2 = prepare_word(&asset_2),
        asset_3 = prepare_word(&asset_3),
        nft = prepare_word(&non_fungible_asset_encoded),
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET),
        asset,
        "asset must be stored at the correct memory location",
    );

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET + 1),
        asset_2_and_3,
        "asset_2 and asset_3 must be stored at the same correct memory location",
    );

    assert_eq!(
        read_root_mem_value(&process, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET + 2),
        Word::from(non_fungible_asset_encoded),
        "non_fungible_asset must be stored at the correct memory location",
    );

    let note_ptr = CREATED_NOTE_SECTION_OFFSET;
    assert_eq!(
        process.stack.get(0),
        Felt::from(note_ptr),
        "top item on the stack is a pointer to the created note"
    );
}

#[test]
fn test_create_note_and_add_same_nft_twice() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
        &TransactionKernel::assembler(),
    );

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
    use.miden::kernels::tx::prologue
    use.miden::tx

    begin
        exec.prologue::prepare_transaction

        push.{recipient}
        push.{PUBLIC_NOTE}
        push.{tag}
        push.{nft}

        exec.tx::create_note
        # => [note_ptr]

        push.{nft} movup.4
        exec.tx::add_asset_to_note
        # => [note_ptr]
    end
    ",
        recipient = prepare_word(&recipient),
        PUBLIC_NOTE = NoteType::Public as u8,
        tag = tag,
        nft = prepare_word(&encoded),
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction);

    assert!(
        process.is_err(),
        "Transaction should have failed because the same NFT is added twice"
    );
}

// HELPER FUNCTIONS
// ================================================================================================

fn read_root_mem_value(process: &Process<MockHost>, addr: u32) -> Word {
    process.get_mem_value(ContextId::root(), addr).unwrap()
}
