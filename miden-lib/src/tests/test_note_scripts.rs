use crate::{MidenLib, SatKernel};
use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler,
};
use crypto::{Felt, StarkField, Word, ONE};
use miden_stdlib::StdLibrary;

use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountVault},
    assets::{Asset, FungibleAsset},
    block::BlockHeader,
    chain::ChainMmr,
    notes::{Note, NoteOrigin, NoteScript},
};

use mock::{
    constants::{
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_SENDER, DEFAULT_ACCOUNT_CODE,
    },
    mock::{
        account::{mock_account_storage, MockAccountType},
        notes::AssetPreservationStatus,
        transaction::mock_inputs_with_existing,
    },
};

use miden_tx::{DataStore, TransactionExecutor};

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<Note>,
}

impl MockDataStore {
    pub fn with_existing(account: Option<Account>, consumed_notes: Option<Vec<Note>>) -> Self {
        let (account, block_header, block_chain, consumed_notes) = mock_inputs_with_existing(
            MockAccountType::StandardExisting,
            AssetPreservationStatus::Preserved,
            account,
            consumed_notes,
        );
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
        }
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<Note>), miden_tx::DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num as u64, self.block_header.block_num().as_int());
        assert_eq!(notes.len(), self.notes.len());
        let origins = self
            .notes
            .iter()
            .map(|note| note.proof().as_ref().unwrap().origin())
            .collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(&note));
        Ok((
            self.account.clone(),
            self.block_header.clone(),
            self.block_chain.clone(),
            self.notes.clone(),
        ))
    }

    fn get_account_code(
        &self,
        account_id: AccountId,
    ) -> Result<ModuleAst, miden_tx::DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}

// We test the Pay to ID script. So we create a note that can
// only be consumed by the target account.
#[test]
fn test_p2id_script() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let target_account_code_src = DEFAULT_ACCOUNT_CODE;
    let target_account_code_ast = ModuleAst::parse(target_account_code_src).unwrap();
    let mut account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let target_account_code =
        AccountCode::new(target_account_code_ast.clone(), &mut account_assembler).unwrap();

    let target_account_storage = mock_account_storage();
    let target_account: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![]).unwrap(),
        target_account_storage.clone(),
        target_account_code.clone(),
        Felt::new(1),
    );

    // Create the note
    let note_script_ast = ProgramAst::parse(
        format!(
            "
        use.miden::note_scripts::basic
    
        begin
            exec.basic::p2id
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    let mut note_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let (note_script, _) = NoteScript::new(note_script_ast, &mut note_assembler).unwrap();

    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    let note = Note::new(
        note_script.clone(),
        &[target_account_id.into()],
        &vec![fungible_asset_1],
        SERIAL_NUM,
        sender_account_id,
        ONE,
        None,
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(target_account), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // nonce delta
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset_1]).unwrap(),
        target_account_storage,
        target_account_code,
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let malicious_account_code_src = DEFAULT_ACCOUNT_CODE;
    let malicious_account_code_ast = ModuleAst::parse(malicious_account_code_src).unwrap();
    let mut malicious_account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let malicious_account_code =
        AccountCode::new(malicious_account_code_ast.clone(), &mut malicious_account_assembler)
            .unwrap();

    let malicious_account_storage = mock_account_storage();
    let malicious_account: Account = Account::new(
        malicious_account_id,
        AccountVault::new(&vec![]).unwrap(),
        malicious_account_storage.clone(),
        malicious_account_code.clone(),
        Felt::new(1),
    );

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());

    executor_2.load_account(malicious_account_id).unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 =
        executor_2.execute_transaction(malicious_account_id, block_ref, &note_origins, None);

    // Check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

// We test the Pay to script with 2 assets to test the loop inside the script.
// So we create a note containing two assets that can only be consumed by the target account.
#[test]
fn test_p2id_script_two_assets() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let target_account_code_src = DEFAULT_ACCOUNT_CODE;
    let target_account_code_ast = ModuleAst::parse(target_account_code_src).unwrap();
    let mut account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let target_account_code =
        AccountCode::new(target_account_code_ast.clone(), &mut account_assembler).unwrap();

    let target_account_storage = mock_account_storage();
    let target_account: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![]).unwrap(),
        target_account_storage.clone(),
        target_account_code.clone(),
        Felt::new(1),
    );

    // Create the note
    let note_script_ast = ProgramAst::parse(
        format!(
            "
        use.miden::note_scripts::basic
    
        begin
            exec.basic::p2id
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    let mut note_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let (note_script, _) = NoteScript::new(note_script_ast, &mut note_assembler).unwrap();

    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    let note = Note::new(
        note_script.clone(),
        &[target_account_id.into()],
        &vec![fungible_asset_1, fungible_asset_2],
        SERIAL_NUM,
        sender_account_id,
        ONE,
        None,
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::with_existing(Some(target_account), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    // Execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // Nonce delta
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset_1, fungible_asset_2]).unwrap(),
        target_account_storage,
        target_account_code,
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Failure)
    // --------------------------------------------------------------------------------------------
    // A "malicious" account tries to consume the note, we expect an error

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let malicious_account_code_src = DEFAULT_ACCOUNT_CODE;
    let malicious_account_code_ast = ModuleAst::parse(malicious_account_code_src).unwrap();
    let mut malicious_account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let malicious_account_code =
        AccountCode::new(malicious_account_code_ast.clone(), &mut malicious_account_assembler)
            .unwrap();

    let malicious_account_storage = mock_account_storage();
    let malicious_account: Account = Account::new(
        malicious_account_id,
        AccountVault::new(&vec![]).unwrap(),
        malicious_account_storage.clone(),
        malicious_account_code.clone(),
        Felt::new(1),
    );

    let data_store_malicious_account =
        MockDataStore::with_existing(Some(malicious_account), Some(vec![note]));
    let mut executor_2 = TransactionExecutor::new(data_store_malicious_account.clone());

    executor_2.load_account(malicious_account_id).unwrap();

    let block_ref = data_store_malicious_account.block_header.block_num().as_int() as u32;
    let note_origins = data_store_malicious_account
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 =
        executor_2.execute_transaction(malicious_account_id, block_ref, &note_origins, None);

    // check that we got the expected result - TransactionExecutorError
    assert!(transaction_result_2.is_err());
}

// We want to test the Pay to ID Reclaim script, which is a script that allows the user
// to provide a block height to the P2ID script. Before the block height is reached,
// the note can only be consumed by the target account. After the block height is reached,
// the note can also be consumed (reclaimed) by the sender account.
#[test]
fn test_p2idr_script() {
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    // Create sender and target and malicious account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let sender_account_code_src = DEFAULT_ACCOUNT_CODE;
    let sender_account_code_ast = ModuleAst::parse(sender_account_code_src).unwrap();
    let mut sender_account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let sender_account_code =
        AccountCode::new(sender_account_code_ast.clone(), &mut sender_account_assembler).unwrap();

    let sender_account_storage = mock_account_storage();

    // Sender account has an empty vault; this is because we only test note consumption, not creation.
    let sender_account: Account = Account::new(
        sender_account_id,
        AccountVault::new(&vec![]).unwrap(),
        sender_account_storage.clone(),
        sender_account_code.clone(),
        Felt::new(1),
    );

    // Now create the target account
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let target_account_code_src = DEFAULT_ACCOUNT_CODE;
    let target_account_code_ast = ModuleAst::parse(target_account_code_src).unwrap();
    let mut target_account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let target_account_code =
        AccountCode::new(target_account_code_ast.clone(), &mut target_account_assembler).unwrap();

    let target_account_storage = mock_account_storage();
    let target_account: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![]).unwrap(),
        target_account_storage.clone(),
        target_account_code.clone(),
        Felt::new(1),
    );

    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN + 1).unwrap();
    let malicious_account_code_src = DEFAULT_ACCOUNT_CODE;
    let malicious_account_code_ast = ModuleAst::parse(malicious_account_code_src).unwrap();
    let mut malicious_account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let malicious_account_code =
        AccountCode::new(malicious_account_code_ast.clone(), &mut malicious_account_assembler)
            .unwrap();

    let malicious_account_storage = mock_account_storage();
    let malicious_account: Account = Account::new(
        malicious_account_id,
        AccountVault::new(&vec![]).unwrap(),
        malicious_account_storage.clone(),
        malicious_account_code.clone(),
        Felt::new(1),
    );

    // --------------------------------------------------------------------------------------------
    // Create notes
    // Create the reclaim block height (Note: Current block height is 4)
    let reclaim_block_height_in_time = Felt::new(5);
    let reclaim_block_height_too_late = Felt::new(3);

    // Create the note with the P2IDR script
    let note_program_ast = ProgramAst::parse(
        format!(
            "
                use.miden::note_scripts::basic
    
                begin
                    exec.basic::p2idr
                end
                ",
        )
        .as_str(),
    )
    .unwrap();

    let (note_script, _) =
        NoteScript::new(note_program_ast, &mut sender_account_assembler).unwrap();

    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    let note_in_time = Note::new(
        note_script.clone(),
        &[target_account_id.into(), reclaim_block_height_in_time],
        &vec![fungible_asset_1],
        SERIAL_NUM,
        sender_account_id,
        ONE,
        None,
    )
    .unwrap();

    let note_too_late = Note::new(
        note_script.clone(),
        &[target_account_id.into(), reclaim_block_height_too_late],
        &vec![fungible_asset_1],
        SERIAL_NUM,
        sender_account_id,
        ONE,
        None,
    )
    .unwrap();

    // --------------------------------------------------------------------------------------------
    // We have two cases:
    //  Case "in time": block height is 4, reclaim block height is 5. Only the target account can consume the note.
    //  Case "too late": block height is 4, reclaim block height is 3. Target and sender account can consume the note.
    //  The malicious account should never be able to consume the note.
    // --------------------------------------------------------------------------------------------
    // CONSTRUCT AND EXECUTE TX (Case "in time" - Target Account Execution Success)
    // --------------------------------------------------------------------------------------------
    let data_store_1 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    let mut executor_1 = TransactionExecutor::new(data_store_1.clone());

    executor_1.load_account(target_account_id).unwrap();

    let block_ref_1 = data_store_1.block_header.block_num().as_int() as u32;
    let note_origins = data_store_1
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let tx_script = ProgramAst::parse(
        format!(
            "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx_rpo_falcon512
        end
        "
        )
        .as_str(),
    )
    .unwrap();

    // Execute the transaction and get the witness
    let transaction_result_1 = executor_1
        .execute_transaction(target_account_id, block_ref_1, &note_origins, Some(tx_script.clone()))
        .unwrap();

    // Assert that the target_account received the funds and the nonce increased by 1
    // Nonce delta
    assert!(transaction_result_1.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset_1]).unwrap(),
        target_account_storage.clone(),
        target_account_code.clone(),
        Felt::new(2),
    );
    assert!(transaction_result_1.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Sender Account Execution Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_2 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    let mut executor_2 = TransactionExecutor::new(data_store_2.clone());

    executor_2.load_account(sender_account_id).unwrap();

    let block_ref_2 = data_store_2.block_header.block_num().as_int() as u32;
    let note_origins_2 = data_store_2
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_2 = executor_2.execute_transaction(
        sender_account_id,
        block_ref_2,
        &note_origins_2,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Second transaction should not work (sender consumes too early), we expect an error
    assert!(transaction_result_2.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Malicious Target Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_3 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_in_time.clone()]),
    );
    let mut executor_3 = TransactionExecutor::new(data_store_3.clone());

    executor_3.load_account(malicious_account_id).unwrap();

    let block_ref_3 = data_store_3.block_header.block_num().as_int() as u32;
    let note_origins_3 = data_store_3
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_3 = executor_3.execute_transaction(
        malicious_account_id,
        block_ref_3,
        &note_origins_3,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Third transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_3.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Execution Target Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_4 = MockDataStore::with_existing(
        Some(target_account.clone()),
        Some(vec![note_too_late.clone()]),
    );
    let mut executor_4 = TransactionExecutor::new(data_store_4.clone());

    executor_4.load_account(target_account_id).unwrap();

    let block_ref_4 = data_store_4.block_header.block_num().as_int() as u32;
    let note_origins_4 = data_store_4
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_4 = executor_4
        .execute_transaction(
            target_account_id,
            block_ref_4,
            &note_origins_4,
            Some(tx_script.clone()),
        )
        .unwrap();

    // Check that we got the expected result - TransactionResult
    // Assert that the target_account received the funds and the nonce increased by 1
    // Nonce delta
    assert!(transaction_result_4.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::new(
        target_account_id,
        AccountVault::new(&vec![fungible_asset_1]).unwrap(),
        target_account_storage,
        target_account_code,
        Felt::new(2),
    );
    assert!(transaction_result_4.final_account_hash() == target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Execution Sender Account Success)
    // --------------------------------------------------------------------------------------------
    let data_store_5 = MockDataStore::with_existing(
        Some(sender_account.clone()),
        Some(vec![note_too_late.clone()]),
    );
    let mut executor_5 = TransactionExecutor::new(data_store_5.clone());

    executor_5.load_account(sender_account_id).unwrap();

    let block_ref_5 = data_store_5.block_header.block_num().as_int() as u32;
    let note_origins = data_store_5
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_5 = executor_5
        .execute_transaction(sender_account_id, block_ref_5, &note_origins, Some(tx_script.clone()))
        .unwrap();

    // Assert that the sender_account received the funds and the nonce increased by 1
    // Nonce delta
    assert!(transaction_result_5.account_delta().nonce == Some(Felt::new(2)));

    // Vault delta (Note: vault was empty before)
    let sender_account_after: Account = Account::new(
        sender_account_id,
        AccountVault::new(&vec![fungible_asset_1]).unwrap(),
        sender_account_storage,
        sender_account_code,
        Felt::new(2),
    );
    assert!(transaction_result_5.final_account_hash() == sender_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Malicious Account Failure)
    // --------------------------------------------------------------------------------------------
    let data_store_6 = MockDataStore::with_existing(
        Some(malicious_account.clone()),
        Some(vec![note_too_late.clone()]),
    );
    let mut executor_6 = TransactionExecutor::new(data_store_6.clone());

    executor_6.load_account(malicious_account_id).unwrap();

    let block_ref_6 = data_store_6.block_header.block_num().as_int() as u32;
    let note_origins_6 = data_store_6
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result_6 = executor_6.execute_transaction(
        malicious_account_id,
        block_ref_6,
        &note_origins_6,
        Some(tx_script.clone()),
    );

    // Check that we got the expected result - TransactionExecutorError and not TransactionResult
    // Sixth transaction should not work (malicious account can never consume), we expect an error
    assert!(transaction_result_6.is_err())
}
