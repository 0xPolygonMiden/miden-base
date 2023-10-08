use super::{
    AccountId, BlockHeader, ChainMmr, DataStore, DataStoreError, Note, NoteOrigin,
    TransactionExecutor, TransactionProver, TransactionVerifier, TryFromVmResult,
};
use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler,
};
use crypto::StarkField;
use miden_objects::{
    accounts::{Account, AccountCode},
    transaction::{CreatedNotes, FinalAccountStub},
};
use miden_prover::ProvingOptions;
use mock::{
    constants::{CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH, CHILD_STORAGE_INDEX_0},
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    utils::prepare_word,
};
use vm_core::Felt;
use vm_processor::MemAdviceProvider;

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<Note>,
}

impl MockDataStore {
    pub fn new() -> Self {
        let (account, block_header, block_chain, consumed_notes) =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
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
        Self::new()
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_data(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<(Account, BlockHeader, ChainMmr, Vec<Note>), DataStoreError> {
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
    ) -> Result<assembly::ast::ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}

#[test]
fn test_transaction_executor_witness() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, None)
        .unwrap();
    let witness = transaction_result.clone().into_witness();

    // use the witness to execute the transaction again
    let mut mem_advice_provider: MemAdviceProvider = witness.advice_inputs().clone().into();
    let result = vm_processor::execute(
        witness.program(),
        witness.get_stack_inputs(),
        &mut mem_advice_provider,
        Default::default(),
    )
    .unwrap();

    let (stack, map, store) = mem_advice_provider.into_parts();
    let final_account_stub =
        FinalAccountStub::try_from_vm_result(result.stack_outputs(), &stack, &map, &store).unwrap();
    let created_notes =
        CreatedNotes::try_from_vm_result(result.stack_outputs(), &stack, &map, &store).unwrap();

    assert_eq!(transaction_result.final_account_hash(), final_account_stub.0.hash());
    assert_eq!(transaction_result.created_notes(), &created_notes);
}

#[test]
#[ignore]
fn test_transaction_result_account_delta() {
    let data_store = MockDataStore::new();
    let account_id = data_store.account.id();

    let new_acct_code_src = "\
    export.account_proc_1
        push.9.9.9.9
        dropw
    end
    ";
    let new_acct_code_ast = ModuleAst::parse(new_acct_code_src).unwrap();
    let new_acct_code =
        AccountCode::new(new_acct_code_ast.clone(), &mut Assembler::default()).unwrap();

    // TODO: This currently has some problems due to stack management when context switching: https://github.com/0xPolygonMiden/miden-base/issues/173
    let tx_script = format!(
        "\
        use.context::account_{account_id}
        use.miden::sat::account

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        #TODO: Move this into an account library
        proc.set_item
            push.0 movdn.5 push.0 movdn.5 push.0 movdn.5
            # => [index, V', 0, 0, 0]

            call.account_{account_id}::set_item
            # => [R', V]
        end

        proc.set_code
            call.account_{account_id}::set_code
            # => [0, 0, 0, 0]

            dropw
            # => []
        end

        proc.incr_nonce
            call.account_{account_id}::incr_nonce
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

            ## Update account code
            ## ------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_ROOT} exec.set_code
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce
        end
    ",
        NEW_ACCOUNT_ROOT = prepare_word(&*new_acct_code.root())
    );
    let tx_script = ProgramAst::parse(&tx_script).unwrap();

    let mut executor = TransactionExecutor::new(data_store.clone());
    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // expected delta

    // execute the transaction and get the witness
    let transaction_result = executor
        .execute_transaction(account_id, block_ref, &note_origins, Some(tx_script))
        .unwrap();

    // nonce delta
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // storage delta
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
}

#[test]
fn test_prove_witness_and_verify() {
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

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
    let data_store = MockDataStore::new();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

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
