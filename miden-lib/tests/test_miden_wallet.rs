pub mod common;

use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler,
};
use common::{data::prepare_word, prepare_transaction, run_tx, MemAdviceProvider, NodeIndex};
use crypto::{
    hash::rpo::RpoDigest as Digest, merkle::MerkleTreeDelta, Felt, StarkField, Word, ONE, ZERO,
};
use miden_lib::{MidenLib, SatKernel};
use miden_stdlib::StdLibrary;
use rand::{self, SeedableRng};
use rand_chacha::ChaCha8Rng;
use vm_core::StackInputs;

use miden_objects::{
    assets::{Asset, FungibleAsset},
    builder::DEFAULT_ACCOUNT_CODE,
    mock::{
        self, mock_account_storage, mock_account_vault, mock_inputs, AssetPreservationStatus,
        Immutable, MockAccountType, MockChain, OnChain, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN, ACCOUNT_ID_SENDER,
    },
    notes::{Note, NoteOrigin, NoteScript},
    Account, AccountCode, AccountId, AccountVault, BlockHeader, ChainMmr,
};

use miden_tx::{data::DataStore, TransactionExecutor};

#[derive(Clone)]
pub enum DataStoreError {
    AccountNotFound(AccountId),
    NoteNotFound(u32, NodeIndex),
}

#[derive(Clone)]

pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<Note>,
}

impl MockDataStore {
    pub fn new(account: Option<Account>, consumed_notes: Option<Vec<Note>>) -> Self {
        let (account, block_header, block_chain, consumed_notes) = mock_inputs(
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

#[test]
// Testing the basic Miden wallet - receiving an asset
fn test_receive_asset_via_wallet() {
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

    let target_account_code = AccountCode::new(
        target_account_id,
        target_account_code_ast.clone(),
        &mut account_assembler,
    )
    .unwrap();

    let target_account: Account = Account::new(
        target_account_id,
        mock_account_vault(),
        mock_account_storage(),
        target_account_code,
        Felt::new(1),
    );

    // Create the note
    let note_script_ast = ProgramAst::parse(
        format!(
            "
    use.miden::sat::note
    use.miden::wallets::basic->wallet

    # add the asset
    begin
        exec.note::get_assets drop
        mem_loadw
        exec.wallet::receive_asset
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
        &[],
        &vec![fungible_asset_1],
        SERIAL_NUM,
        sender_account_id,
        ONE,
        None,
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::new(Some(target_account), Some(vec![note]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let transaction_result =
        executor.execute_transaction(target_account_id, block_ref, &note_origins, None);

    println!("transaction_result: {:?}", transaction_result.unwrap_err());

    // // nonce delta
    // assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // // vault delta
    // assert!(transaction_result.account_delta().vault == MerkleTreeDelta::new(1));
}

#[test]
// Testing the basic Miden wallet - sending an asset
fn test_send_asset_via_wallet() {
    // Mock data
    // We need an account that owns an asset

    let mut mock_chain = MockChain::new(ChaCha8Rng::seed_from_u64(0)).unwrap();

    // Create the faucet
    let faucet_id = mock_chain
        .new_fungible_faucet(OnChain::Yes, DEFAULT_ACCOUNT_CODE, Digest::default())
        .unwrap();

    // Create an asset
    let asset = FungibleAsset::new(faucet_id, 100).unwrap();

    // Create the account
    mock_chain
        .new_account(
            include_str!("../asm/sat/account.masm"),
            vec![],
            vec![asset.clone().try_into().unwrap()],
            Immutable::No,
            OnChain::No,
        )
        .unwrap();

    // Seal the block
    let block_header = mock_chain.seal_block().unwrap();

    // Create the transaction
    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let transaction_script: String = format!(
        "
    use.miden::wallets::basic->wallet

    begin
        push.{recipient}
        push.{tag}
        push.{asset}
        exec.wallet::send_asset
    end
        ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&asset.try_into().unwrap())
    );

    // FIX: change prepare_transaction to accept references
    let transaction = prepare_transaction(
        mock_chain.account_mut(0).clone(),
        None,
        block_header,
        mock_chain.chain().clone(),
        vec![],
        &transaction_script,
        "",
        None,
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    // ToDo: check the account has the asset not anymore

    // ToDo: check that there is a note with the asset
}
