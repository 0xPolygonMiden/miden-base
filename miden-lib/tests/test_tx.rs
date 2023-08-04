pub mod common;
use assembly::ast::{ModuleAst, ProgramAst};
use common::{
    data::{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_SENDER},
    memory::{
        CREATED_NOTE_ASSETS_OFFSET, CREATED_NOTE_METADATA_OFFSET, CREATED_NOTE_RECIPIENT_OFFSET,
        CREATED_NOTE_SECTION_OFFSET, NUM_CREATED_NOTES_PTR,
    },
    procedures::prepare_word,
    run_within_tx_kernel, Felt, MemAdviceProvider, Note, NoteTarget, StackInputs,
    TransactionComplier, ONE, ZERO,
};
use crypto::{FieldElement, Word};
use miden_objects::{
    assets::{Asset, FungibleAsset},
    mock::{mock_inputs, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN},
    AccountId,
};

#[test]
fn test_create_note() {
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
    use.miden::sat::tx

    begin
        push.{recipient}
        push.{tag}
        push.{asset}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset)
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        StackInputs::default(),
        MemAdviceProvider::default(),
        None,
        None,
    )
    .unwrap();

    // assert the number of created notes has been incremented to 1.
    assert_eq!(
        process.get_memory_value(0, NUM_CREATED_NOTES_PTR).unwrap(),
        [ONE, ZERO, ZERO, ZERO]
    );

    // assert the recipient is stored at the correct memory location.
    assert_eq!(
        process
            .get_memory_value(0, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_RECIPIENT_OFFSET)
            .unwrap(),
        recipient
    );

    // assert the metadata is stored at the correct memory location.
    assert_eq!(
        process
            .get_memory_value(0, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_METADATA_OFFSET)
            .unwrap(),
        [ONE, tag, ZERO, ZERO]
    );

    // assert the asset is stored at the correct memory location.
    assert_eq!(
        process
            .get_memory_value(0, CREATED_NOTE_SECTION_OFFSET + CREATED_NOTE_ASSETS_OFFSET)
            .unwrap(),
        asset
    );

    // assert there top item on the stack is a pointer to the created note.
    assert_eq!(process.stack.get(0), Felt::new(10000));
}

#[test]
fn test_create_note_too_many_notes() {
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);
    let asset = [Felt::new(10), ZERO, ZERO, Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)];

    let code = format!(
        "
    use.miden::sat::internal::constants
    use.miden::sat::internal::layout
    use.miden::sat::tx

    begin
        exec.constants::get_max_num_created_notes
        exec.layout::set_num_created_notes

        push.{recipient}
        push.{tag}
        push.{asset}

        exec.tx::create_note
    end
    ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset)
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        StackInputs::default(),
        MemAdviceProvider::default(),
        None,
        None,
    );

    // assert the process failed
    assert!(process.is_err());
}

#[test]
fn test_p2id_script() {
    let mut tx_compiler = TransactionComplier::new();

    // Sender account Id
    let sender_account = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Create target account and load into tx compiler
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    const ACCOUNT_CODE_MASM: &'static str = "\
        export.add_asset
            push.99
            drop
        end
        ";
    let target_account_code_ast = ModuleAst::parse(ACCOUNT_CODE_MASM).unwrap();
    let _account_code =
        tx_compiler.load_account(target_account_id, target_account_code_ast).unwrap();

    // create note script
    let note_program_ast =
        ProgramAst::parse(format!("use.context::account_{target_account_id} begin call.account_{target_account_id}::add_asset end", ).as_str()).unwrap();
    let note_script = tx_compiler
        .compile_note_script(note_program_ast, vec![NoteTarget::AccountId(target_account_id)])
        .unwrap();

    // Create Note and all assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    // create a note
    let note = Note::new(
        note_script.clone(),
        &[Felt::new(1)],
        &vec![fungible_asset_1],
        SERIAL_NUM_1,
        sender_account,
        Felt::ZERO,
        None,
    )
    .unwrap();

    // Now I want that target_account consumes the note

    // Then I want to play around with the note script and finally create my Pay 2 ID script
}
