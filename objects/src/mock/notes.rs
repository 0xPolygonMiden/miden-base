use crate::mock::{
    prepare_assets, prepare_word, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
};

use super::super::{
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteScript},
    AccountId, Felt, Vec, Word,
};
use assembly::{ast::ProgramAst, Assembler};
use miden_core::FieldElement;

pub fn mock_consumed_notes(assembler: &mut Assembler, created_notes: &[Note]) -> Vec<Note> {
    // Note Assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 200).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 300).unwrap().into();

    // Sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // create note 1 script
    let note_1_script_src = format!(
        "\
        use.miden::sat::tx

        begin
            # create note 0
            push.{created_note_0_recipient}
            push.{created_note_0_tag}
            push.{created_note_0_asset}
            exec.tx::create_note
            drop

            # create note 1
            push.{created_note_1_recipient}
            push.{created_note_1_tag}
            push.{created_note_1_asset}
            exec.tx::create_note
            drop
        end
    ",
        created_note_0_recipient = prepare_word(&created_notes[0].recipient()),
        created_note_0_tag = created_notes[0].metadata().tag(),
        created_note_0_asset = prepare_assets(created_notes[0].vault())[0],
        created_note_1_recipient = prepare_word(&created_notes[1].recipient()),
        created_note_1_tag = created_notes[1].metadata().tag(),
        created_note_1_asset = prepare_assets(created_notes[1].vault())[0],
    );
    let note_1_script_ast = ProgramAst::parse(&note_1_script_src).unwrap();
    let (note_1_script, _) = NoteScript::new(note_1_script_ast, assembler).unwrap();

    // create note 2 script
    let note_2_script_src = format!(
        "\
        use.miden::sat::tx


        begin
            # create note 2
            push.{created_note_2_recipient}
            push.{created_note_2_tag}
            push.{created_note_2_asset}
            exec.tx::create_note
            drop
        end
        ",
        created_note_2_recipient = prepare_word(&created_notes[2].recipient()),
        created_note_2_tag = created_notes[2].metadata().tag(),
        created_note_2_asset = prepare_assets(created_notes[2].vault())[0],
    );
    let note_2_script_ast = ProgramAst::parse(&note_2_script_src).unwrap();
    let (note_2_script, _) = NoteScript::new(note_2_script_ast, assembler).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_1 = Note::new(
        note_1_script,
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_2 = Note::new(
        note_2_script,
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2]
}

pub fn mock_created_notes(assembler: &mut Assembler) -> Vec<Note> {
    // Note assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 100).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 100).unwrap().into();

    // sender account (account transaction is executed against)
    let sender = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();

    // create note script
    let note_program_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_script, _) = NoteScript::new(note_program_ast, assembler).unwrap();

    // Created Notes
    const SERIAL_NUM_1: Word = [Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)];
    let note_1 = Note::new(
        note_script.clone(),
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let note_2 = Note::new(
        note_script.clone(),
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_3: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let note_3 = Note::new(
        note_script,
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_3,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2, note_3]
}
