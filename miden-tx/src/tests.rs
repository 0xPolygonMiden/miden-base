use alloc::vec::Vec;

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{
        Account, AccountCode, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
    },
    assembly::{Assembler, ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    block::BlockHeader,
    notes::{NoteId, NoteType},
    transaction::{
        ChainMmr, InputNote, InputNotes, ProvenTransaction, TransactionArgs, TransactionWitness,
    },
    Felt, Word,
};
use miden_prover::ProvingOptions;
use mock::{
    constants::{non_fungible_asset, FUNGIBLE_ASSET_AMOUNT, MIN_PROOF_SECURITY_LEVEL},
    mock::{
        account::{
            MockAccountType, ACCOUNT_INCR_NONCE_MAST_ROOT, ACCOUNT_SET_CODE_MAST_ROOT,
            ACCOUNT_SET_ITEM_MAST_ROOT, STORAGE_INDEX_0,
        },
        notes::AssetPreservationStatus,
        transaction::mock_inputs,
    },
    utils::prepare_word,
};
use vm_processor::{
    utils::{Deserializable, Serializable},
    MemAdviceProvider,
};

use super::{
    AccountId, DataStore, DataStoreError, TransactionExecutor, TransactionHost, TransactionInputs,
    TransactionProver, TransactionVerifier,
};

// TESTS
// ================================================================================================

#[test]
fn transaction_executor_witness() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, data_store.tx_args().clone())
        .unwrap();
    let tx_witness: TransactionWitness = executed_transaction.clone().into();

    // use the witness to execute the transaction again
    let (stack_inputs, advice_inputs) = tx_witness.get_kernel_inputs();
    let mem_advice_provider: MemAdviceProvider = advice_inputs.into();
    let mut host = TransactionHost::new(tx_witness.account().into(), mem_advice_provider);
    let result =
        vm_processor::execute(tx_witness.program(), stack_inputs, &mut host, Default::default())
            .unwrap();

    let (advice_provider, _, output_notes) = host.into_parts();
    let (_, map, _) = advice_provider.into_parts();
    let tx_outputs = TransactionKernel::from_transaction_parts(
        result.stack_outputs(),
        &map.into(),
        output_notes,
    )
    .unwrap();

    assert_eq!(executed_transaction.final_account().hash(), tx_outputs.account.hash());
    assert_eq!(executed_transaction.output_notes(), &tx_outputs.output_notes);
}

#[test]
fn executed_transaction_account_delta() {
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
    let new_acct_code = AccountCode::new(new_acct_code_ast.clone(), &Assembler::default()).unwrap();

    // updated storage
    let updated_slot_value = [Felt::new(7), Felt::new(9), Felt::new(11), Felt::new(13)];

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
    let removed_assets = [removed_asset_1, removed_asset_2, removed_asset_3];

    let tx_script = format!(
        "\
        use.miden::account
        use.miden::contracts::wallets::basic->wallet

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        #TODO: Move this into an account library
        proc.set_item
            push.0 movdn.5 push.0 movdn.5 push.0 movdn.5
            # => [index, V', 0, 0, 0]

            call.{ACCOUNT_SET_ITEM_MAST_ROOT}
            # => [R', V]
        end

        proc.set_code
            call.{ACCOUNT_SET_CODE_MAST_ROOT}
            # => [0, 0, 0, 0]

            dropw
            # => []
        end

        proc.incr_nonce
            call.{ACCOUNT_INCR_NONCE_MAST_ROOT}
            # => [0]

            drop
            # => []
        end

        ## TRANSACTION SCRIPT
        ## ========================================================================================
        begin
            ## Update account storage item
            ## ------------------------------------------------------------------------------------
            # push a new value for the storage slot onto the stack
            push.{UPDATED_SLOT_VALUE}
            # => [13, 11, 9, 7]

            # get the index of account storage slot
            push.{STORAGE_INDEX_0}
            # => [idx, 13, 11, 9, 7]

            # update the storage value
            exec.set_item dropw dropw
            # => []

            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{OFFCHAIN}         # note_type
            push.999                # tag
            push.{REMOVED_ASSET_1}  # asset
            call.wallet::send_asset dropw dropw drop drop
            # => []

            # totally deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{OFFCHAIN}         # note_type
            push.998                # tag
            push.{REMOVED_ASSET_2}  # asset
            call.wallet::send_asset dropw dropw drop drop
            # => []

            # send non-fungible asset
            push.0.1.2.3            # recipient
            push.{OFFCHAIN}         # note_type
            push.997                # tag
            push.{REMOVED_ASSET_3}  # asset
            call.wallet::send_asset dropw dropw drop drop
            # => []

            ## Update account code
            ## ------------------------------------------------------------------------------------
            push.{NEW_ACCOUNT_ROOT} exec.set_code dropw
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce drop
            # => []
        end
    ",
        NEW_ACCOUNT_ROOT = prepare_word(&new_acct_code.root()),
        UPDATED_SLOT_VALUE = prepare_word(&Word::from(updated_slot_value)),
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        OFFCHAIN = NoteType::OffChain as u8,
    );
    let tx_script_code = ProgramAst::parse(&tx_script).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();
    let tx_args =
        TransactionArgs::new(Some(tx_script), None, data_store.tx_args.advice_map().clone());

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // nonce delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().nonce(), Some(Felt::new(2)));

    // storage delta
    // --------------------------------------------------------------------------------------------
    assert_eq!(executed_transaction.account_delta().storage().updated_items.len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].0,
        STORAGE_INDEX_0
    );
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].1,
        updated_slot_value
    );

    // vault delta
    // --------------------------------------------------------------------------------------------
    // assert that added assets are tracked
    let added_assets = data_store
        .notes
        .last()
        .unwrap()
        .note()
        .assets()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(executed_transaction
        .account_delta()
        .vault()
        .added_assets
        .iter()
        .all(|x| added_assets.contains(x)));
    assert_eq!(
        added_assets.len(),
        executed_transaction.account_delta().vault().added_assets.len()
    );

    // assert that removed assets are tracked
    assert!(executed_transaction
        .account_delta()
        .vault()
        .removed_assets
        .iter()
        .all(|x| removed_assets.contains(x)));
    assert_eq!(
        removed_assets.len(),
        executed_transaction.account_delta().vault().removed_assets.len()
    );
}

#[test]
fn prove_witness_and_verify() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone());

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, data_store.tx_args().clone())
        .unwrap();

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);
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

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

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
    let tx_args =
        TransactionArgs::new(Some(tx_script), None, data_store.tx_args.advice_map().clone());

    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args);

    assert!(
        executed_transaction.is_ok(),
        "Transaction execution failed {:?}",
        executed_transaction,
    );
}

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<InputNote>,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation: AssetPreservationStatus) -> Self {
        let (tx_inputs, tx_args) =
            mock_inputs(MockAccountType::StandardExisting, asset_preservation);
        let (account, _, block_header, block_chain, notes) = tx_inputs.into_parts();

        Self {
            account,
            block_header,
            block_chain,
            notes: notes.into_vec(),
            tx_args,
        }
    }

    fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new(AssetPreservationStatus::Preserved)
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num, self.block_header.block_num());
        assert_eq!(notes.len(), self.notes.len());

        let notes = self
            .notes
            .iter()
            .filter(|note| notes.contains(&note.id()))
            .cloned()
            .collect::<Vec<_>>();

        Ok(TransactionInputs::new(
            self.account.clone(),
            None,
            self.block_header,
            self.block_chain.clone(),
            InputNotes::new(notes).unwrap(),
        )
        .unwrap())
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
