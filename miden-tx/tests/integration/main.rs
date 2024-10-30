extern crate alloc;

mod scripts;
mod wallet;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_SENDER, Account, AccountCode, AccountId, AccountStorage,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::{dsa::rpo_falcon512::SecretKey, utils::Serializable},
    notes::{Note, NoteAssets, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteType},
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{ExecutedTransaction, ProvenTransaction, TransactionArgs, TransactionScript},
    Felt, Word, ZERO,
};
use miden_prover::ProvingOptions;
use miden_tx::{
    LocalTransactionProver, TransactionProver, TransactionVerifier, TransactionVerifierError,
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use vm_processor::utils::Deserializable;

// HELPER FUNCTIONS
// ================================================================================================

#[cfg(test)]
pub fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    let executed_transaction_id = executed_transaction.id();
    // Prove the transaction

    let proof_options = ProvingOptions::default();
    let prover = LocalTransactionProver::new(proof_options);
    let proven_transaction = prover.prove(executed_transaction.into()).unwrap();

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
) -> (Word, std::sync::Arc<dyn miden_tx::auth::TransactionAuthenticator>) {
    use alloc::sync::Arc;

    use miden_objects::accounts::AuthSecretKey;
    use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
    use rand::rngs::StdRng;

    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Arc::new(authenticator) as Arc<dyn TransactionAuthenticator>)
}

#[cfg(test)]
pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    use miden_objects::{
        accounts::{AccountComponent, AssembledAccountComponent, StorageMap, StorageSlot},
        crypto::dsa::rpo_falcon512::PublicKey,
        testing::account_component::{RpoFalcon512, BASIC_WALLET_CODE},
    };
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let wallet_component = AssembledAccountComponent::compile(
        BASIC_WALLET_CODE,
        assembler.clone(),
        vec![StorageSlot::Value(Word::default()), StorageSlot::Map(StorageMap::default())],
    )
    .unwrap();

    let account_components = [
        RpoFalcon512::new(PublicKey::new(public_key))
            .assemble_component(assembler)
            .unwrap(),
        wallet_component,
    ];
    let account_code = AccountCode::from_components(&account_components).unwrap();
    let account_storage = AccountStorage::from_components(&account_components).unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

#[cfg(test)]
pub fn get_note_with_fungible_asset_and_script(
    fungible_asset: FungibleAsset,
    note_script: &str,
) -> Note {
    use miden_objects::notes::NoteExecutionHint;

    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let note_script = NoteScript::compile(note_script, assembler).unwrap();
    const SERIAL_NUM: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let sender_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let vault = NoteAssets::new(vec![fungible_asset.into()]).unwrap();
    let metadata =
        NoteMetadata::new(sender_id, NoteType::Public, 1.into(), NoteExecutionHint::Always, ZERO)
            .unwrap();
    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(SERIAL_NUM, note_script, inputs);

    Note::new(vault, metadata, recipient)
}

#[cfg(test)]
pub fn build_default_auth_script() -> TransactionScript {
    TransactionScript::compile(DEFAULT_AUTH_SCRIPT, [], TransactionKernel::assembler()).unwrap()
}

#[cfg(test)]
pub fn build_tx_args_from_script(script_source: &str) -> TransactionArgs {
    let tx_script =
        TransactionScript::compile(script_source, [], TransactionKernel::assembler()).unwrap();
    TransactionArgs::with_tx_script(tx_script)
}
