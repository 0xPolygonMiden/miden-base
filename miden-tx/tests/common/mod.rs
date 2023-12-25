use miden_lib::assembler::assembler;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AccountVault, StorageSlotType},
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, FungibleAsset},
    crypto::{dsa::rpo_falcon512::KeyPair, utils::Serializable},
    notes::{Note, NoteOrigin, NoteScript, RecordedNote},
    transaction::{ChainMmr, TransactionInputs},
    BlockHeader, Felt, Word,
};
use miden_tx::{DataStore, DataStoreError};
use mock::{
    constants::{ACCOUNT_ID_SENDER, DEFAULT_ACCOUNT_CODE},
    mock::{
        account::MockAccountType,
        notes::AssetPreservationStatus,
        transaction::{mock_inputs, mock_inputs_with_existing},
    },
};
use vm_processor::AdviceInputs;

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<RecordedNote>,
    pub auxiliary_data: AdviceInputs,
}

impl MockDataStore {
    pub fn new() -> Self {
        let (account, block_header, block_chain, consumed_notes, auxiliary_data) =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
            auxiliary_data,
        }
    }

    pub fn with_existing(
        account: Option<Account>,
        consumed_notes: Option<Vec<Note>>,
        auxiliary_data: Option<AdviceInputs>,
    ) -> Self {
        let (account, block_header, block_chain, consumed_notes, mut auxiliary_data_inputs) =
            mock_inputs_with_existing(
                MockAccountType::StandardExisting,
                AssetPreservationStatus::Preserved,
                account,
                consumed_notes,
            );
        if let Some(auxiliary_data) = auxiliary_data {
            auxiliary_data_inputs.extend(auxiliary_data);
        }
        Self {
            account,
            block_header,
            block_chain,
            notes: consumed_notes,
            auxiliary_data: auxiliary_data_inputs,
        }
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteOrigin],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num, self.block_header.block_num());
        assert_eq!(notes.len(), self.notes.len());
        let origins = self.notes.iter().map(|note| note.origin()).collect::<Vec<_>>();
        notes.iter().all(|note| origins.contains(&note));
        Ok(TransactionInputs {
            account: self.account.clone(),
            account_seed: None,
            block_header: self.block_header,
            block_chain: self.block_chain.clone(),
            input_notes: self.notes.clone(),
            aux_data: self.auxiliary_data.clone(),
        })
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}

// HELPER FUNCTIONS
// ================================================================================================
pub fn get_new_key_pair_with_advice_map() -> (Word, Vec<Felt>) {
    let keypair: KeyPair = KeyPair::new().unwrap();

    let pk: Word = keypair.public_key().into();
    let pk_sk_bytes = keypair.to_bytes();
    let pk_sk_felts: Vec<Felt> =
        pk_sk_bytes.iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>();

    (pk, pk_sk_felts)
}

#[allow(dead_code)]
pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage =
        AccountStorage::new(vec![(0, (StorageSlotType::Value { value_arity: 0 }, public_key))])
            .unwrap();

    let account_vault = match assets {
        Some(asset) => AccountVault::new(&[asset]).unwrap(),
        None => AccountVault::new(&[]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

#[allow(dead_code)]
pub fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: ProgramAst,
) -> Note {
    let note_assembler = assembler();
    let (note_script, _) = NoteScript::new(note_script, &note_assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    Note::new(
        note_script.clone(),
        &[],
        &[fungible_asset.into()],
        SERIAL_NUM,
        sender_id,
        Felt::new(1),
    )
    .unwrap()
}
