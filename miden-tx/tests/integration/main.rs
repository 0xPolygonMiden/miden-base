mod scripts;
mod wallet;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_SENDER, Account, AccountCode, AccountId, AccountStorage,
        SlotItem,
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
use winter_maybe_async::maybe_async;

// HELPER FUNCTIONS
// ================================================================================================

#[cfg(test)]
pub fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    let executed_transaction_id = executed_transaction.id();
    // Prove the transaction

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    // Serialize & deserialize the ProvenTransaction
    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    // Verify that the generated proof is valid
    let verifier = TransactionVerifier::new(miden_objects::MIN_PROOF_SECURITY_LEVEL);

    verifier.verify(proven_transaction)
}

#[cfg(test)]
pub fn get_new_pk_and_authenticator(
) -> (Word, std::rc::Rc<miden_tx::auth::BasicAuthenticator<rand::rngs::StdRng>>) {
    use std::rc::Rc;

    use miden_objects::accounts::AuthSecretKey;
    use miden_tx::auth::BasicAuthenticator;
    use rand::rngs::StdRng;

    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Rc::new(authenticator))
}

#[cfg(test)]
pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    use std::collections::BTreeMap;

    use miden_objects::testing::account_code::DEFAULT_ACCOUNT_CODE;
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let account_code_ast = ModuleAst::parse(account_code_src).unwrap();
    let account_assembler = TransactionKernel::assembler();

    let account_code = AccountCode::new(account_code_ast.clone(), &account_assembler).unwrap();
    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, public_key)], BTreeMap::new()).unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(account_id, account_vault, account_storage, account_code, Felt::new(1))
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
