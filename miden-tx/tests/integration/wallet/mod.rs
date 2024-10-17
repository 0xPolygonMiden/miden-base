use miden_lib::{accounts::wallets::create_basic_wallet, AuthScheme};
use miden_objects::{accounts::AccountId, crypto::dsa::rpo_falcon512::SecretKey, Word};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

use crate::get_account_with_default_account_code;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn wallet_creation() {
    use miden_objects::accounts::{
        account_id::testing::ACCOUNT_ID_SENDER, AccountStorageMode, AccountType,
    };

    // we need a Falcon Public Key to create the wallet account
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key = sec_key.public_key();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

    // we need to use an initial seed to create the wallet account
    let init_seed: [u8; 32] = [
        95, 113, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let account_type = AccountType::RegularAccountImmutableCode;
    let storage_mode = AccountStorageMode::Private;

    let (wallet, _) =
        create_basic_wallet(init_seed, auth_scheme, account_type, storage_mode).unwrap();

    // sender_account_id not relevant here, just to create a default account code
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let expected_code_commitment =
        get_account_with_default_account_code(sender_account_id, pub_key.into(), None)
            .code()
            .commitment();

    assert!(wallet.is_regular_account());
    assert_eq!(wallet.code().commitment(), expected_code_commitment);
    let pub_key_word: Word = pub_key.into();
    assert_eq!(wallet.storage().get_item(0).unwrap().as_elements(), pub_key_word);
}
