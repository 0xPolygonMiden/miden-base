pub mod common;

use assembly::{
    ast::{ModuleAst, ProgramAst},
    Assembler,
};
use common::{data::prepare_word, NodeIndex};
use crypto::{
    Felt, StarkField, Word, ONE, ZERO,
};
use miden_lib::{MidenLib, SatKernel};
use miden_stdlib::StdLibrary;



use miden_objects::{
    assets::{Asset, FungibleAsset},
    builder::DEFAULT_ACCOUNT_CODE,
    mock::{
        mock_account_storage, mock_inputs, AssetPreservationStatus, MockAccountType, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
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
    use.miden::sat::note
    use.miden::wallets::basic->wallet

    # add the asset
    begin
        exec.note::get_assets drop
        mem_loadw
        call.wallet::receive_asset
        dropw
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

    let tx_script = ProgramAst::parse(
        format!(
        "
        use.miden::eoa::basic->auth_tx

        begin
            call.auth_tx::auth_tx
        end
        ").as_str()).unwrap();
    // Execute the transaction and get the witness
    let transaction_result =
        executor.execute_transaction(target_account_id, block_ref, &note_origins, Some(tx_script)).unwrap();

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
}

#[test]
// Testing the basic Miden wallet - sending an asset
fn test_send_asset_via_wallet() {
    // Mock data
    // We need an asset and an account that owns that asset
    // Create assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();

    // Create sender and target account
    let sender_account_id =
        AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let sender_account_code_src = DEFAULT_ACCOUNT_CODE;
    let sender_account_code_ast = ModuleAst::parse(sender_account_code_src).unwrap();
    let mut account_assembler = Assembler::default()
        .with_library(&MidenLib::default())
        .expect("library is well formed")
        .with_library(&StdLibrary::default())
        .expect("library is well formed")
        .with_kernel(SatKernel::kernel())
        .expect("kernel is well formed");

    let sender_account_code = AccountCode::new(
        sender_account_id,
        sender_account_code_ast.clone(),
        &mut account_assembler,
    )
    .unwrap();

    let sender_account_storage = mock_account_storage();
    let sender_account: Account = Account::new(
        sender_account_id,
        AccountVault::new(&vec![fungible_asset_1.clone()]).unwrap(),
        sender_account_storage.clone(),
        sender_account_code.clone(),
        Felt::new(1),
    );

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store = MockDataStore::new(Some(sender_account), None);

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(sender_account_id).unwrap();

    let block_ref = data_store.block_header.block_num().as_int() as u32;
    let note_origins = data_store
        .notes
        .iter()
        .map(|note| note.proof().as_ref().unwrap().origin().clone())
        .collect::<Vec<_>>();

    let recipient = [ZERO, ONE, Felt::new(2), Felt::new(3)];
    let tag = Felt::new(4);

    let tx_script = ProgramAst::parse(
        format!(
        "
        use.miden::eoa::basic->auth_tx
        use.miden::wallets::basic->wallet

        begin
            push.{recipient}
            push.{tag}
            push.{asset}
            call.wallet::send_asset drop
            call.auth_tx::auth_tx
            dropw dropw 
        end
        ",
        recipient = prepare_word(&recipient),
        tag = tag,
        asset = prepare_word(&fungible_asset_1.try_into().unwrap())
        ).as_str()).unwrap();
    
    // Execute the transaction and get the witness
    let transaction_result =
        executor.execute_transaction(sender_account_id, block_ref, &note_origins, Some(tx_script)).unwrap();
    

    // nonce delta
    assert!(transaction_result.account_delta().nonce == Some(Felt::new(2)));

    // vault delta
    let sender_account_after: Account = Account::new(
        sender_account_id,
        AccountVault::new(&vec![]).unwrap(),
        sender_account_storage,
        sender_account_code,
        Felt::new(2),
    );
    assert!(transaction_result.final_account_hash() == sender_account_after.hash());

}
