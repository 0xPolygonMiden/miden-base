use super::{
    Account, AccountId, BlockHeader, ChainMmr, DataStore, DataStoreError, NoteOrigin,
    TransactionExecutor, TransactionHost, TransactionProver, TransactionVerifier, TryFromVmResult,
};
use miden_objects::{
    accounts::AccountCode,
    assembly::{Assembler, ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    notes::RecordedNote,
    transaction::{CreatedNotes, FinalAccountStub},
    Felt, StarkField, Word,
};
use miden_prover::ProvingOptions;
use mock::{
    constants::{
        non_fungible_asset, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX, ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX,
        ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX, CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH,
        CHILD_STORAGE_INDEX_0, FUNGIBLE_ASSET_AMOUNT,
    },
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    utils::prepare_word,
};
use vm_core::utils::to_hex;
use vm_processor::MemAdviceProvider;

// TESTS
// ================================================================================================

#[test]
fn test_transaction_executor_witness() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // use the witness to execute the transaction again
    let mem_advice_provider: MemAdviceProvider = witness.advice_inputs().clone().into();
    let mut host = TransactionHost::new(mem_advice_provider);
    let result = vm_processor::execute(
        witness.program(),
        witness.get_stack_inputs(),
        &mut host,
        Default::default(),
    )
    .unwrap();

    let (advice_provider, _event_handler) = host.into_parts();
    let (stack, map, store) = advice_provider.into_parts();
    let final_account_stub =
        FinalAccountStub::try_from_vm_result(result.stack_outputs(), &stack, &map, &store).unwrap();
    let created_notes =
        CreatedNotes::try_from_vm_result(result.stack_outputs(), &stack, &map, &store).unwrap();

    assert_eq!(transaction_result.final_account_hash(), final_account_stub.0.hash());
    assert_eq!(transaction_result.created_notes(), &created_notes);
}

#[test]
fn test_transaction_result_account_delta() {
    let data_store = MockDataStore::new(AssetPreservationStatus::PreservedWithAccountVaultDelta);
    let mut executor = TransactionExecutor::new(data_store.clone());
    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let new_acct_code_src = "\
    export.account_proc_1
        push.9.9.9.9
        dropw
    end
    ";
    let new_acct_code_ast = ModuleAst::parse(new_acct_code_src).unwrap();
    let new_acct_code =
        AccountCode::new(new_acct_code_ast.clone(), &mut Assembler::default()).unwrap();

    // removed assets
    let removed_asset_1 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let removed_asset_2 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let removed_assets = vec![removed_asset_1, removed_asset_2, removed_asset_3];

    let account_procedure_incr_nonce_mast_root = to_hex(
        &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_INCR_NONCE_PROC_IDX].as_bytes(),
    )
    .unwrap();
    let account_procedure_set_code_mast_root = to_hex(
        &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_SET_CODE_PROC_IDX].as_bytes(),
    )
    .unwrap();
    let account_procedure_set_item_mast_root = to_hex(
        &data_store.account.code().procedures()[ACCOUNT_PROCEDURE_SET_ITEM_PROC_IDX].as_bytes(),
    )
    .unwrap();

    let tx_script = format!(
        "\
        use.miden::miden::sat::account
        use.miden::miden::wallets::basic->wallet

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        #TODO: Move this into an account library
        proc.set_item
            push.0 movdn.5 push.0 movdn.5 push.0 movdn.5
            # => [index, V', 0, 0, 0]

            call.0x{account_procedure_set_item_mast_root}
            # => [R', V]
        end

        proc.set_code
            call.0x{account_procedure_set_code_mast_root}
            # => [0, 0, 0, 0]

            dropw
            # => []
        end

        proc.incr_nonce
            call.0x{account_procedure_incr_nonce_mast_root}
            # => [0]

            drop
            # => []
        end

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Update account storage child tree
            ## ------------------------------------------------------------------------------------
            # get the current child tree root from account storage slot
            push.{CHILD_ROOT_PARENT_LEAF_INDEX}
            # => [idx]

            # get the child root
            exec.account::get_item
            # => [CHILD_ROOT]

            # prepare the stack to remove in the child tree
            padw swapw push.0 push.{CHILD_SMT_DEPTH}
            # => [depth, idx(push.0), CHILD_ROOT, NEW_VALUE (padw)]

            # set new value and drop old value
            mtree_set dropw
            # => [NEW_CHILD_ROOT]

            # prepare stack to delete existing child tree value (replace with empty word)
            padw swapw push.{CHILD_STORAGE_INDEX_0} push.{CHILD_SMT_DEPTH}
            # => [depth, idx, NEW_CHILD_ROOT, EMPTY_WORD]

            # set existing value to empty word
            mtree_set dropw
            # => [NEW_CHILD_ROOT]

            # store the new child root in account storage slot
            push.{CHILD_ROOT_PARENT_LEAF_INDEX} exec.set_item dropw dropw
            # => []

            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3
            push.999
            push.{REMOVED_ASSET_1}
            call.wallet::send_asset drop dropw dropw

            # totally deplete fungible asset balance
            push.0.1.2.3
            push.999
            push.{REMOVED_ASSET_2}
            call.wallet::send_asset drop dropw dropw

            # send non-fungible asset
            push.0.1.2.3
            push.999
            push.{REMOVED_ASSET_3}
            call.wallet::send_asset drop dropw dropw

            ## Update account code
            ## ------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_ROOT} exec.set_code
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce
        end
    ",
        NEW_ACCOUNT_ROOT = prepare_word(&*new_acct_code.root()),
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
    );
    let tx_script_code = ProgramAst::parse(&tx_script).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(transaction_result.account_delta().storage.slots_delta.updated_slots().len(), 1);
    assert_eq!(
        transaction_result.account_delta().storage.slots_delta.updated_slots()[0].0,
        CHILD_ROOT_PARENT_LEAF_INDEX as u64
    );
    assert_eq!(transaction_result.account_delta().storage.store_delta.0.len(), 1);
    assert_eq!(
        transaction_result.account_delta().storage.store_delta.0[0].1.cleared_slots()[0],
        CHILD_STORAGE_INDEX_0
    );

    // vault delta
    // --------------------------------------------------------------------------------------------
    // assert that added assets are tracked
    let added_assets = data_store
        .notes
        .last()
        .unwrap()
        .note()
        .vault()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(transaction_result
        .account_delta()
        .vault
        .added_assets
        .iter()
        .all(|x| added_assets.contains(x)));
    assert_eq!(added_assets.len(), transaction_result.account_delta().vault.added_assets.len());

    // assert that removed assets are tracked
    assert!(transaction_result
        .account_delta()
        .vault
        .removed_assets
        .iter()
        .all(|x| removed_assets.contains(x)));
    assert_eq!(
        removed_assets.len(),
        transaction_result.account_delta().vault.removed_assets.len()
    );
}

#[test]
fn test_prove_witness_and_verify() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // prove the transaction with the witness
    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction_witness(witness).unwrap();

    let verifier = TransactionVerifier::new(96);
    assert!(verifier.verify(proven_transaction).is_ok());
}

#[test]
fn test_prove_and_verify_with_tx_executor() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    // prove the transaction with the executor
    let prepared_transaction = executor
        .prepare_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();

    // prove transaction
    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_prepared_transaction(prepared_transaction).unwrap();

    let verifier = TransactionVerifier::new(96);
    assert!(verifier.verify(proven_transaction).is_ok());
}

// TEST TRANSACTION SCRIPT
// ================================================================================================

#[test]
fn test_tx_script() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins =
        data_store.notes.iter().map(|note| note.origin().clone()).collect::<Vec<_>>();

    let tx_script_input_key = [Felt::new(9999), Felt::new(8888), Felt::new(9999), Felt::new(8888)];
    let tx_script_input_value = [Felt::new(9), Felt::new(8), Felt::new(7), Felt::new(6)];
    let tx_script_source = format!(
        "
    begin
        # push the tx script input key onto the stack
        push.{key}

        # load the tx script input value from the map and read it onto the stack
        adv.push_mapval adv_loadw

        # assert that the value is correct
        push.{value} assert_eqw
    end
",
        key = prepare_word(&tx_script_input_key),
        value = prepare_word(&tx_script_input_value)
    );
    let tx_script_code = ProgramAst::parse(&tx_script_source).unwrap();
    let tx_script = executor
        .compile_tx_script(
            tx_script_code,
            vec![(tx_script_input_key, tx_script_input_value.into())],
            vec![],
        )
        .unwrap();

    // execute the transaction
    let transaction_result =
        executor.execute_transaction(account_id, block_ref, &note_origins, Some(tx_script));

    // assert the transaction executed successfully
    assert!(transaction_result.is_ok());
}

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<RecordedNote>,
}

impl MockDataStore {
    pub fn new(asset_preservation: AssetPreservationStatus) -> Self {
        let (account, block_header, block_chain, consumed_notes) =
            mock_inputs(MockAccountType::StandardExisting, asset_preservation);
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new(AssetPreservationStatus::Preserved)
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<RecordedNote>), DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num as u64, self.block_header.block_num().as_int());
        assert_eq!(notes.len(), self.notes.len());
        let origins = self.notes.iter().map(|note| note.origin()).collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(&note));
        Ok((
            self.account.clone(),
            self.block_header,
            self.block_chain.clone(),
            self.notes.clone(),
        ))
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
