use alloc::vec::Vec;

use miden_objects::{
    accounts::{
        AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_SENDER,
    },
    assembly::{Assembler, ProgramAst},
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteAssets, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteType},
    transaction::OutputNote,
    Felt, Word, ZERO,
};

use crate::{
    constants::{
        non_fungible_asset_2, CONSUMED_ASSET_1_AMOUNT, CONSUMED_ASSET_2_AMOUNT,
        CONSUMED_ASSET_3_AMOUNT,
    },
    mock::account::ACCOUNT_CREATE_NOTE_MAST_ROOT,
    utils::{prepare_assets, prepare_word},
};

pub enum AssetPreservationStatus {
    TooFewInput,
    Preserved,
    PreservedWithAccountVaultDelta,
    TooManyFungibleInput,
    TooManyNonFungibleInput,
}

pub fn mock_notes(
    assembler: &Assembler,
    asset_preservation: &AssetPreservationStatus,
) -> (Vec<Note>, Vec<OutputNote>) {
    let mut serial_num_gen = SerialNumGenerator::new();

    // ACCOUNT IDS
    // --------------------------------------------------------------------------------------------
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

    // ASSETS
    // --------------------------------------------------------------------------------------------
    let fungible_asset_1: Asset =
        FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
    let fungible_asset_2: Asset =
        FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
    let fungible_asset_3: Asset =
        FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();

    // CREATED NOTES
    // --------------------------------------------------------------------------------------------
    let note_program_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_script, _) = NoteScript::new(note_program_ast, assembler).unwrap();

    let inputs = NoteInputs::new(vec![Felt::new(1)]).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_1]).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_script.clone(), inputs);
    let created_note_1 = Note::new(vault, metadata, recipient);

    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_2]).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_script.clone(), inputs);
    let created_note_2 = Note::new(vault, metadata, recipient);

    let inputs = NoteInputs::new(vec![Felt::new(3)]).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_3]).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_script.clone(), inputs);
    let created_note_3 = Note::new(vault, metadata, recipient);

    // CONSUMED NOTES
    // --------------------------------------------------------------------------------------------
    let note_1_script_src = format!(
        "\
        begin
            # create note 0
            push.{recipient0}
            push.{PUBLIC_NOTE}
            push.{tag0}
            push.{asset0}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            drop drop dropw dropw

            # create note 1
            push.{recipient1}
            push.{PUBLIC_NOTE}
            push.{tag1}
            push.{asset1}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            drop drop dropw dropw
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        recipient0 = prepare_word(&created_note_1.recipient_digest()),
        tag0 = created_note_1.metadata().tag(),
        asset0 = prepare_assets(created_note_1.assets())[0],
        recipient1 = prepare_word(&created_note_2.recipient_digest()),
        tag1 = created_note_2.metadata().tag(),
        asset1 = prepare_assets(created_note_2.assets())[0],
    );
    let note_1_script_ast = ProgramAst::parse(&note_1_script_src).unwrap();
    let (note_1_script, _) = NoteScript::new(note_1_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_1]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(1)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_1_script, inputs);
    let consumed_note_1 = Note::new(vault, metadata, recipient);

    let note_2_script_src = format!(
        "\
        begin
            # create note 2
            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{tag}
            push.{asset}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}
            drop drop dropw dropw
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        recipient = prepare_word(&created_note_3.recipient_digest()),
        tag = created_note_3.metadata().tag(),
        asset = prepare_assets(created_note_3.assets())[0],
    );
    let note_2_script_ast = ProgramAst::parse(&note_2_script_src).unwrap();
    let (note_2_script, _) = NoteScript::new(note_2_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_2, fungible_asset_3]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_2_script, inputs);
    let consumed_note_2 = Note::new(vault, metadata, recipient);

    let note_3_script_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_3_script, _) = NoteScript::new(note_3_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_2, fungible_asset_3]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_3_script, inputs);
    let consumed_note_3 = Note::new(vault, metadata, recipient);

    let note_4_script_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_4_script, _) = NoteScript::new(note_4_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault =
        NoteAssets::new(vec![non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN)])
            .unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(1)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_4_script, inputs);
    let consumed_note_4 = Note::new(vault, metadata, recipient);

    // note that changes the account vault
    let note_5_script_ast = ProgramAst::parse(
        "\
        use.miden::note
        use.miden::contracts::wallets::basic->wallet

        begin
            # read the assets to memory
            push.0 exec.note::get_assets
            # => [num_assets, dest_ptr]

            # assert the number of assets is 3
            push.3 assert_eq
            # => [dest_ptr]

            # add the first asset to the vault
            padw dup.4 mem_loadw call.wallet::receive_asset dropw
            # => [dest_ptr]

            # add the second asset to the vault
            push.1 add padw dup.4 mem_loadw call.wallet::receive_asset dropw
            # => [dest_ptr+1]

            # add the third asset to the vault
            push.1 add padw movup.4 mem_loadw call.wallet::receive_asset dropw
            # => []
        end
        ",
    )
    .unwrap();
    let (note_5_script, _) = NoteScript::new(note_5_script_ast, assembler).unwrap();

    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![
        fungible_asset_1,
        fungible_asset_3,
        non_fungible_asset_2(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN),
    ])
    .unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_5_script, inputs);
    let consumed_note_5 = Note::new(vault, metadata, recipient);

    let consumed_notes = match asset_preservation {
        AssetPreservationStatus::TooFewInput => vec![consumed_note_1],
        AssetPreservationStatus::Preserved => {
            vec![consumed_note_1, consumed_note_2]
        },
        AssetPreservationStatus::PreservedWithAccountVaultDelta => {
            vec![consumed_note_1, consumed_note_2, consumed_note_5]
        },
        AssetPreservationStatus::TooManyFungibleInput => {
            vec![consumed_note_1, consumed_note_2, consumed_note_3]
        },
        AssetPreservationStatus::TooManyNonFungibleInput => {
            vec![consumed_note_1, consumed_note_2, consumed_note_4]
        },
    };
    let created_notes = vec![
        OutputNote::Public(created_note_1),
        OutputNote::Public(created_note_2),
        OutputNote::Public(created_note_3),
    ];

    (consumed_notes, created_notes)
}

struct SerialNumGenerator {
    state: u64,
}

impl SerialNumGenerator {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn next(&mut self) -> Word {
        let serial_num = [
            Felt::new(self.state),
            Felt::new(self.state + 1),
            Felt::new(self.state + 2),
            Felt::new(self.state + 3),
        ];
        self.state += 4;
        serial_num
    }
}
