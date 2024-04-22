mod scripts;
mod wallet;

use miden_lib::transaction::TransactionKernel;
use miden_mock::{constants::MIN_PROOF_SECURITY_LEVEL, mock::account::DEFAULT_ACCOUNT_CODE};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, SlotItem, StorageSlot, ACCOUNT_ID_SENDER,
    },
    assembly::{ModuleAst, ProgramAst},
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::SecretKey, utils::Serializable},
    notes::{Note, NoteAssets, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteType},
    transaction::{ExecutedTransaction, ProvenTransaction},
    Felt, Word, ZERO,
};
use miden_prover::ProvingOptions;
use miden_tx::{TransactionProver, TransactionVerifier, TransactionVerifierError};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use vm_processor::utils::Deserializable;

// HELPER FUNCTIONS
// ================================================================================================

#[cfg(test)]
pub fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    // Prove the transaction
    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    // Serialize & deserialize the ProvenTransaction
    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    // Verify that the generated proof is valid
    let verifier = TransactionVerifier::new(MIN_PROOF_SECURITY_LEVEL);

    verifier.verify(proven_transaction)
}

#[cfg(test)]
pub fn get_new_key_pair_with_advice_map() -> (Word, Vec<Felt>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();
    let mut pk_sk_bytes = sec_key.to_bytes();
    pk_sk_bytes.append(&mut pub_key.to_bytes());
    let pk_sk_felts: Vec<Felt> =
        pk_sk_bytes.iter().map(|a| Felt::new(*a as u64)).collect::<Vec<Felt>>();

    (pub_key, pk_sk_felts)
}

#[cfg(test)]
pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage = AccountStorage::new(
        vec![SlotItem {
            index: 0,
            slot: StorageSlot::new_value(public_key),
        }],
        vec![],
    )
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::new(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

#[cfg(test)]
pub fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: ProgramAst,
) -> Note {
    let note_assembler = TransactionKernel::assembler();
    let (note_script, _) = NoteScript::new(note_script, &note_assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let vault = NoteAssets::new(vec![fungible_asset.into()]).unwrap();
    let metadata = NoteMetadata::new(sender_id, NoteType::Public, 1.into(), ZERO).unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(SERIAL_NUM, note_script, inputs);

    Note::new(vault, metadata, recipient)
}
