#[cfg(not(feature = "sync"))]
use alloc::boxed::Box;
use alloc::vec::Vec;

use maybe_async::maybe_async;
use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        },
        Account, AccountCode,
    },
    assembly::{Assembler, ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    block::BlockHeader,
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteHeader, NoteId, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transaction::{
        ChainMmr, InputNote, InputNotes, ProvenTransaction, TransactionArgs, TransactionWitness,
    },
    Felt, Word, ZERO,
};
use miden_prover::ProvingOptions;
use mock::{
    constants::{non_fungible_asset, FUNGIBLE_ASSET_AMOUNT, MIN_PROOF_SECURITY_LEVEL},
    mock::{
        account::{
            MockAccountType, ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT, ACCOUNT_CREATE_NOTE_MAST_ROOT,
            ACCOUNT_INCR_NONCE_MAST_ROOT, ACCOUNT_REMOVE_ASSET_MAST_ROOT,
            ACCOUNT_SET_CODE_MAST_ROOT, ACCOUNT_SET_ITEM_MAST_ROOT, ACCOUNT_SET_MAP_ITEM_MAST_ROOT,
            STORAGE_INDEX_0, STORAGE_INDEX_2,
        },
        notes::AssetPreservationStatus,
        transaction::mock_inputs,
    },
    utils::prepare_word,
};
use vm_processor::{
    utils::{Deserializable, Serializable},
    Digest, MemAdviceProvider,
};

use super::{
    AccountId, DataStore, DataStoreError, TransactionExecutor, TransactionHost, TransactionInputs,
    TransactionProver, TransactionVerifier,
};

// TESTS
// ================================================================================================

#[maybe_async::test(feature = "sync")]
fn transaction_executor_witness() {
    let data_store = MockDataStore::default();
    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None);

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
    let _authenticator = ();
    let mut host: TransactionHost<MemAdviceProvider, ()> =
        TransactionHost::new(tx_witness.account().into(), mem_advice_provider, None);
    let result =
        vm_processor::execute(tx_witness.program(), stack_inputs, &mut host, Default::default())
            .unwrap();

    let (advice_provider, _, output_notes, _signatures) = host.into_parts();
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

#[maybe_async::test(feature = "sync")]
fn executed_transaction_account_delta() {
    let data_store = MockDataStore::new(AssetPreservationStatus::PreservedWithAccountVaultDelta);
    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None);
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

    // updated storage map
    let updated_map_key = [Felt::new(14), Felt::new(15), Felt::new(16), Felt::new(17)];
    let updated_map_value = [Felt::new(18), Felt::new(19), Felt::new(20), Felt::new(21)];

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

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionHint::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_local_use_case(0, 0).unwrap();
    let tag3 = NoteTag::for_local_use_case(0, 0).unwrap();

    let note_type1 = NoteType::OffChain;
    let note_type2 = NoteType::OffChain;
    let note_type3 = NoteType::OffChain;

    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    assert_eq!(tag2.validate(note_type2), Ok(tag2));
    assert_eq!(tag3.validate(note_type3), Ok(tag3));

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

        proc.set_map_item
            #push.0 movdn.9 push.0 movdn.9 push.0 movdn.9
            # => [index, KEY, VALUE, 0, 0, 0]

            call.{ACCOUNT_SET_MAP_ITEM_MAST_ROOT}
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

            ## Update account storage map
            ## ------------------------------------------------------------------------------------
            # push a new VALUE for the storage map onto the stack
            push.{UPDATED_MAP_VALUE}
            # => [18, 19, 20, 21]

            # push a new KEY for the storage map onto the stack
            push.{UPDATED_MAP_KEY}
            # => [14, 15, 16, 17, 18, 19, 20, 21]

            # get the index of account storage slot
            push.{STORAGE_INDEX_2}
            # => [idx, 14, 15, 16, 17, 18, 19, 20, 21]

            # update the storage value
            exec.set_map_item dropw dropw dropw
            # => []

            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{NOTETYPE1}        # note_type
            push.{tag1}             # tag
            push.{REMOVED_ASSET_1}  # asset
            call.wallet::send_asset dropw dropw dropw dropw
            # => []

            # totally deplete fungible asset balance
            push.0.1.2.3            # recipient
            push.{NOTETYPE2}        # note_type
            push.{tag2}             # tag
            push.{REMOVED_ASSET_2}  # asset
            call.wallet::send_asset dropw dropw dropw dropw
            # => []

            # send non-fungible asset
            push.0.1.2.3            # recipient
            push.{NOTETYPE3}        # note_type
            push.{tag3}             # tag
            push.{REMOVED_ASSET_3}  # asset
            call.wallet::send_asset dropw dropw dropw dropw
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
        UPDATED_MAP_VALUE = prepare_word(&Word::from(updated_map_value)),
        UPDATED_MAP_KEY = prepare_word(&Word::from(updated_map_key)),
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
        NOTETYPE3 = note_type3 as u8,
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
    assert_eq!(executed_transaction.account_delta().storage().updated_items.len(), 2);
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].0,
        STORAGE_INDEX_0
    );
    assert_eq!(
        executed_transaction.account_delta().storage().updated_items[0].1,
        updated_slot_value
    );

    assert_eq!(executed_transaction.account_delta().storage().updated_maps.len(), 1);
    assert_eq!(
        executed_transaction.account_delta().storage().updated_maps[0].0,
        STORAGE_INDEX_2
    );
    assert_eq!(
        executed_transaction.account_delta().storage().updated_maps[0].1.updated_leaves[0],
        (updated_map_key, updated_map_value)
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

#[maybe_async::test(feature = "sync")]
fn executed_transaction_output_notes() {
    let data_store = MockDataStore::new(AssetPreservationStatus::PreservedWithAccountVaultDelta);
    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None).with_debug_mode(true);
    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

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
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );
    let combined_asset = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT,
        )
        .expect("asset is valid"),
    );
    let removed_asset_3 = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let removed_asset_4 = Asset::Fungible(
        FungibleAsset::new(
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2.try_into().expect("id is valid"),
            FUNGIBLE_ASSET_AMOUNT / 2,
        )
        .expect("asset is valid"),
    );

    let tag1 = NoteTag::from_account_id(
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN.try_into().unwrap(),
        NoteExecutionHint::Local,
    )
    .unwrap();
    let tag2 = NoteTag::for_public_use_case(0, 0, NoteExecutionHint::Local).unwrap();

    let note_type1 = NoteType::OffChain;
    let note_type2 = NoteType::Public;

    assert_eq!(tag1.validate(note_type1), Ok(tag1));
    assert_eq!(tag2.validate(note_type2), Ok(tag2));

    // create the expected output note
    let serial_num = Word::from([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]);
    let note_program_ast = ProgramAst::parse("begin push.1 drop end").unwrap();
    let (note_script, _) = NoteScript::new(note_program_ast, &Assembler::default()).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let metadata = NoteMetadata::new(account_id, note_type2, tag2, ZERO).unwrap();
    let vault = NoteAssets::new(vec![removed_asset_3, removed_asset_4]).unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    let expected_output_note = Note::new(vault, metadata, recipient);
    let tx_script = format!(
        "\
        use.miden::account
        use.miden::contracts::wallets::basic->wallet

        ## ACCOUNT PROCEDURE WRAPPERS
        ## ========================================================================================
        #TODO: Move this into an account library
        proc.create_note
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}

            swapw dropw swapw dropw swapw dropw
            # => [note_ptr]
        end

        proc.add_asset_to_note
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            swapw dropw
            # => [note_ptr]
        end

        proc.remove_asset
        call.{ACCOUNT_REMOVE_ASSET_MAST_ROOT}
        # => [note_ptr]
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
            ## Send some assets from the account vault
            ## ------------------------------------------------------------------------------------
            # partially deplete fungible asset balance
            push.0.1.2.3                        # recipient
            push.{NOTETYPE1}                    # note_type
            push.{tag1}                         # tag
            exec.create_note
            # => [note_ptr]

            push.{REMOVED_ASSET_1}              # asset
            exec.remove_asset
            movup.4 exec.add_asset_to_note
            # => [note_ptr]


            push.{REMOVED_ASSET_2}              # asset_2
            exec.remove_asset
            # => [ASSET, note_ptr]
            movup.4 exec.add_asset_to_note drop
            # => []

            # send non-fungible asset
            push.{RECIPIENT2}                   # recipient
            push.{NOTETYPE2}                    # note_type
            push.{tag2}                         # tag
            exec.create_note
            # => [note_ptr]

            push.{REMOVED_ASSET_3}              # asset_3
            exec.remove_asset
            movup.4 exec.add_asset_to_note
            # => [note_ptr]

            push.{REMOVED_ASSET_4}              # asset_4
            exec.remove_asset
            # => [ASSET, note_ptr]
            movup.4 exec.add_asset_to_note drop
            # => []

            ## Update the account nonce
            ## ------------------------------------------------------------------------------------
            push.1 exec.incr_nonce drop
            # => []
        end
    ",
        REMOVED_ASSET_1 = prepare_word(&Word::from(removed_asset_1)),
        REMOVED_ASSET_2 = prepare_word(&Word::from(removed_asset_2)),
        REMOVED_ASSET_3 = prepare_word(&Word::from(removed_asset_3)),
        REMOVED_ASSET_4 = prepare_word(&Word::from(removed_asset_4)),
        RECIPIENT2 = prepare_word(&Word::from(expected_output_note.recipient().digest())),
        NOTETYPE1 = note_type1 as u8,
        NOTETYPE2 = note_type2 as u8,
    );
    let tx_script_code = ProgramAst::parse(&tx_script).unwrap();
    let tx_script = executor.compile_tx_script(tx_script_code, vec![], vec![]).unwrap();
    let mut tx_args =
        TransactionArgs::new(Some(tx_script), None, data_store.tx_args.advice_map().clone());

    tx_args.add_expected_output_note(&expected_output_note);

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    // expected delta
    // --------------------------------------------------------------------------------------------
    // execute the transaction and get the witness
    let executed_transaction =
        executor.execute_transaction(account_id, block_ref, &note_ids, tx_args).unwrap();

    // output notes
    // --------------------------------------------------------------------------------------------
    let output_notes = executed_transaction.output_notes();

    // assert that the expected output note is present
    // for some reason we always create 3 output notes when we use the MockDataStore
    // there is already an issue to change that
    assert_eq!(output_notes.num_notes(), 5);

    let created_note_id_3 = executed_transaction.output_notes().get_note(3).id();
    let recipient_3 = Digest::from([Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)]);
    let note_assets_3 = NoteAssets::new(vec![combined_asset]).unwrap();
    let expected_note_id_3 = NoteId::new(recipient_3, note_assets_3.commitment());
    assert_eq!(created_note_id_3, expected_note_id_3);

    // assert that the expected output note 2 is present
    let created_note = executed_transaction.output_notes().get_note(4);
    let note_id = expected_output_note.id();
    let note_metadata = expected_output_note.metadata();
    assert_eq!(NoteHeader::from(created_note), NoteHeader::new(note_id, *note_metadata));
}

#[maybe_async::test(feature = "sync")]
fn prove_witness_and_verify() {
    let data_store = MockDataStore::default();
    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None);

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let executed_transaction = executor
        .execute_transaction(account_id, block_ref, &note_ids, data_store.tx_args().clone())
        .unwrap();
    let executed_transaction_id = executed_transaction.id();

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);
    assert!(verifier.verify(proven_transaction).is_ok());
}

// TEST TRANSACTION SCRIPT
// ================================================================================================

#[maybe_async::test(feature = "sync")]
fn test_tx_script() {
    let data_store = MockDataStore::default();
    let mut executor: TransactionExecutor<_, ()> =
        TransactionExecutor::new(data_store.clone(), None);

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
struct MockDataStore {
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

#[maybe_async]
impl DataStore for MockDataStore {
    async fn get_transaction_inputs(
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

    async fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
