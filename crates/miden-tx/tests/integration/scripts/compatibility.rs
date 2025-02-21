use assembly::LibraryPath;
use miden_lib::{
    account::{
        auth::RpoFalcon512,
        interface::{AccountInterface, CheckResult},
        wallets::BasicWallet,
    },
    note::{create_p2id_note, create_p2idr_note, create_swap_note},
    transaction::TransactionKernel,
};
use miden_objects::{
    account::{AccountBuilder, AccountComponent},
    asset::{FungibleAsset, NonFungibleAsset},
    block::BlockNumber,
    crypto::{
        dsa::rpo_falcon512::SecretKey,
        rand::{FeltRng, RpoRandomCoin},
    },
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    testing::account_id::{
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2,
    },
    Felt,
};
use miden_tx::testing::{Auth, MockChain};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

// DEFAULT NOTES
// ================================================================================================

#[test]
fn test_basic_wallet_default_notes() {
    // STATELESS CHECK
    // --------------------------------------------------------------------------------------------

    let mut mock_chain = MockChain::new();
    let wallet_account =
        mock_chain.add_existing_wallet(Auth::BasicAuth, vec![FungibleAsset::mock(20)]);
    let wallet_account_interface = AccountInterface::from(&wallet_account);

    let faucet_account = mock_chain.add_existing_faucet(Auth::BasicAuth, "POL", 200u64, None);
    let faucet_account_interface = AccountInterface::from(faucet_account.account());

    let p2id_note = create_p2id_note(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap(),
        vec![FungibleAsset::mock(10)],
        NoteType::Public,
        Default::default(),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let p2idr_note = create_p2idr_note(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap(),
        vec![FungibleAsset::mock(10)],
        NoteType::Public,
        Default::default(),
        BlockNumber::default(),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    let offered_asset = NonFungibleAsset::mock(&[5, 6, 7, 8]);
    let requested_asset = NonFungibleAsset::mock(&[1, 2, 3, 4]);

    let (swap_note, _) = create_swap_note(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // Basic wallet
    assert_eq!(CheckResult::Yes, wallet_account_interface.can_consume(&p2id_note));
    assert_eq!(CheckResult::Yes, wallet_account_interface.can_consume(&p2idr_note));
    assert_eq!(CheckResult::Yes, wallet_account_interface.can_consume(&swap_note));

    // Basic fungible faucet
    assert_eq!(CheckResult::No, faucet_account_interface.can_consume(&p2id_note));
    assert_eq!(CheckResult::No, faucet_account_interface.can_consume(&p2idr_note));
    assert_eq!(CheckResult::No, faucet_account_interface.can_consume(&swap_note));

    // STATEFUL CHECK
    // --------------------------------------------------------------------------------------------

    // TODO: implement
}

// CUSTOM NOTES
// ================================================================================================

#[test]
fn test_basic_wallet_custom_notes() {
    // STATELESS CHECK
    // --------------------------------------------------------------------------------------------

    let mut mock_chain = MockChain::new();
    let wallet_account =
        mock_chain.add_existing_wallet(Auth::BasicAuth, vec![FungibleAsset::mock(20)]);
    let wallet_account_interface = AccountInterface::from(&wallet_account);

    let sender_account_id =
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap();
    let serial_num =
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
    let tag = NoteTag::from_account_id(wallet_account.id(), NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        sender_account_id,
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Default::default(),
    )
    .unwrap();
    let vault = NoteAssets::new(vec![FungibleAsset::mock(100)]).unwrap();

    let compatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::faucets::basic_fungible->fungible_faucet

        begin
            push.1
            if.true 
                call.fungible_faucet::distribute
                call.fungible_faucet::burn
            else
                call.wallet::receive_asset
                call.wallet::create_note
                call.wallet::move_asset_to_note
            end
        end
    ";
    let note_script =
        NoteScript::compile(compatible_source_code, TransactionKernel::testing_assembler())
            .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let compatible_custom_note = Note::new(vault.clone(), metadata, recipient);
    assert_eq!(
        CheckResult::Maybe,
        wallet_account_interface.can_consume(&compatible_custom_note)
    );

    let incompatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::faucets::basic_fungible->fungible_faucet

        begin
            push.1
            if.true 
                call.fungible_faucet::distribute
                call.fungible_faucet::burn
            else
                call.fungible_faucet::distribute
                call.wallet::create_note
                call.wallet::move_asset_to_note
            end
        end
    ";
    let note_script =
        NoteScript::compile(incompatible_source_code, TransactionKernel::testing_assembler())
            .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let incompatible_custom_note = Note::new(vault, metadata, recipient);
    assert_eq!(CheckResult::No, wallet_account_interface.can_consume(&incompatible_custom_note));

    // STATEFUL CHECK
    // --------------------------------------------------------------------------------------------

    // TODO: implement
}

#[test]
fn test_basic_fungible_faucet_custom_notes() {
    // STATELESS CHECK
    // --------------------------------------------------------------------------------------------

    let mut mock_chain = MockChain::new();
    let faucet_account = mock_chain.add_existing_faucet(Auth::BasicAuth, "POL", 200u64, None);
    let faucet_account_interface = AccountInterface::from(faucet_account.account());

    let sender_account_id =
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2.try_into().unwrap();
    let serial_num =
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
    let tag = NoteTag::from_account_id(faucet_account.id(), NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        sender_account_id,
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Default::default(),
    )
    .unwrap();
    let vault = NoteAssets::new(vec![FungibleAsset::mock(100)]).unwrap();

    let compatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::faucets::basic_fungible->fungible_faucet

        begin
            push.1
            if.true 
                call.fungible_faucet::distribute
                call.fungible_faucet::burn
            else
                call.wallet::receive_asset
                call.wallet::create_note
                call.wallet::move_asset_to_note
            end
        end
    ";
    let note_script =
        NoteScript::compile(compatible_source_code, TransactionKernel::testing_assembler())
            .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let compatible_custom_note = Note::new(vault.clone(), metadata, recipient);
    assert_eq!(
        CheckResult::Maybe,
        faucet_account_interface.can_consume(&compatible_custom_note)
    );

    let incompatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::faucets::basic_fungible->fungible_faucet

        begin
            push.1
            if.true 
                call.fungible_faucet::distribute
                call.wallet::receive_asset
            else
                call.fungible_faucet::burn
                call.wallet::create_note
                call.wallet::move_asset_to_note
            end
        end
    ";
    let note_script =
        NoteScript::compile(incompatible_source_code, TransactionKernel::testing_assembler())
            .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let incompatible_custom_note = Note::new(vault, metadata, recipient);
    assert_eq!(CheckResult::No, faucet_account_interface.can_consume(&incompatible_custom_note));

    // STATEFUL CHECK
    // --------------------------------------------------------------------------------------------

    // TODO: implement
}

#[test]
fn test_custom_account_custom_notes() {
    // STATELESS CHECK
    // --------------------------------------------------------------------------------------------

    let account_custom_code_source = "
        export.procedure_1
            push.1.2.3.4 dropw
        end

        export.procedure_2
            push.5.6.7.8 dropw
        end
    ";

    let account_component = AccountComponent::compile_with_path(
        account_custom_code_source,
        TransactionKernel::testing_assembler(),
        vec![],
        LibraryPath::new("test::account::component_1").unwrap(),
    )
    .unwrap()
    .with_supports_all_types();

    let target_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(account_component.clone())
        .build_existing()
        .unwrap();
    let target_account_interface = AccountInterface::from(&target_account);

    let mut mock_chain = MockChain::with_accounts(&[target_account.clone()]);
    let sender_account =
        mock_chain.add_existing_wallet(Auth::BasicAuth, vec![FungibleAsset::mock(20)]);
    let serial_num =
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
    let tag = NoteTag::from_account_id(target_account.id(), NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        sender_account.id(),
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Default::default(),
    )
    .unwrap();
    let vault = NoteAssets::new(vec![FungibleAsset::mock(100)]).unwrap();

    let compatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.test::account::component_1->test_account

        begin
            push.1
            if.true 
                call.wallet::receive_asset
                call.test_account::procedure_1
            else
                call.test_account::procedure_1
                call.test_account::procedure_2
            end
        end
    ";
    let note_script = NoteScript::compile(
        compatible_source_code,
        TransactionKernel::testing_assembler()
            .with_library(account_component.library())
            .unwrap(),
    )
    .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let compatible_custom_note = Note::new(vault.clone(), metadata, recipient);
    assert_eq!(
        CheckResult::Maybe,
        target_account_interface.can_consume(&compatible_custom_note)
    );

    let incompatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.test::account::component_1->test_account

        begin
            push.1
            if.true 
                call.wallet::receive_asset
                call.test_account::procedure_1
            else
                call.test_account::procedure_2
                call.wallet::create_note
                call.wallet::move_asset_to_note
            end
        end
    ";
    let note_script = NoteScript::compile(
        incompatible_source_code,
        TransactionKernel::testing_assembler()
            .with_library(account_component.library())
            .unwrap(),
    )
    .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let incompatible_custom_note = Note::new(vault, metadata, recipient);
    assert_eq!(CheckResult::No, target_account_interface.can_consume(&incompatible_custom_note));

    // STATEFUL CHECK
    // --------------------------------------------------------------------------------------------

    // TODO: implement
}

#[test]
fn test_custom_account_multiple_components_custom_notes() {
    // STATELESS CHECK
    // --------------------------------------------------------------------------------------------

    let account_custom_code_source = "
        export.procedure_1
            push.1.2.3.4 dropw
        end

        export.procedure_2
            push.5.6.7.8 dropw
        end
    ";

    let custom_component = AccountComponent::compile_with_path(
        account_custom_code_source,
        TransactionKernel::testing_assembler(),
        vec![],
        LibraryPath::new("test::account::component_1").unwrap(),
    )
    .unwrap()
    .with_supports_all_types();

    let mut rng = ChaCha20Rng::from_seed(Default::default());
    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key = sec_key.public_key();

    let rpo_component = RpoFalcon512::new(pub_key);

    let target_account = AccountBuilder::new(ChaCha20Rng::from_entropy().gen())
        .with_component(custom_component.clone())
        .with_component(BasicWallet)
        .with_component(rpo_component)
        .build_existing()
        .unwrap();
    let target_account_interface = AccountInterface::from(&target_account);

    let mut mock_chain = MockChain::with_accounts(&[target_account.clone()]);
    let sender_account =
        mock_chain.add_existing_wallet(Auth::BasicAuth, vec![FungibleAsset::mock(20)]);
    let serial_num =
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]).draw_word();
    let tag = NoteTag::from_account_id(target_account.id(), NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        sender_account.id(),
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Default::default(),
    )
    .unwrap();
    let vault = NoteAssets::new(vec![FungibleAsset::mock(100)]).unwrap();

    let compatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::auth::basic->basic_auth
        use.test::account::component_1->test_account

        begin
            push.1
            if.true 
                call.wallet::receive_asset
                call.wallet::create_note
                call.wallet::move_asset_to_note
                call.test_account::procedure_1
                call.test_account::procedure_2
                call.basic_auth::auth_tx_rpo_falcon512
            else
                call.test_account::procedure_1
                call.test_account::procedure_2
            end
        end
    ";
    let note_script = NoteScript::compile(
        compatible_source_code,
        TransactionKernel::testing_assembler()
            .with_library(custom_component.library())
            .unwrap(),
    )
    .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let compatible_custom_note = Note::new(vault.clone(), metadata, recipient);
    assert_eq!(
        CheckResult::Maybe,
        target_account_interface.can_consume(&compatible_custom_note)
    );

    let incompatible_source_code = "
        use.miden::contracts::wallets::basic->wallet
        use.miden::contracts::auth::basic->basic_auth
        use.test::account::component_1->test_account
        use.miden::contracts::faucets::basic_fungible->fungible_faucet

        begin
            push.1
            if.true 
                call.wallet::receive_asset
                call.wallet::create_note
                call.wallet::move_asset_to_note
                call.test_account::procedure_1
                call.test_account::procedure_2
                call.basic_auth::auth_tx_rpo_falcon512
                call.fungible_faucet::distribute
            else
                call.test_account::procedure_1
                call.test_account::procedure_2
                call.fungible_faucet::burn
            end
        end
    ";
    let note_script = NoteScript::compile(
        incompatible_source_code,
        TransactionKernel::testing_assembler()
            .with_library(custom_component.library())
            .unwrap(),
    )
    .unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, NoteInputs::default());
    let incompatible_custom_note = Note::new(vault.clone(), metadata, recipient);
    assert_eq!(CheckResult::No, target_account_interface.can_consume(&incompatible_custom_note));
}
