extern crate alloc;
pub use alloc::collections::BTreeMap;
use rand::rngs::StdRng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{from_str, to_string_pretty, Value};
use std::rc::Rc;

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountCode, AccountId, AccountStorage, AuthSecretKey, SlotItem},
    assets::{Asset, AssetVault},
    crypto::dsa::rpo_falcon512::SecretKey,
    transaction::TransactionMeasurements,
    Felt, Word,
};
use miden_tx::auth::BasicAuthenticator;

use super::{read_to_string, write, Benchmark, Path};

// CONSTANTS
// ================================================================================================

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0x200000000000001F; // 2305843009213693983
pub const ACCOUNT_ID_SENDER: u64 = 0x800000000000001F; // 9223372036854775839
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0x900000000000003F; // 10376293541461622847

pub const DEFAULT_AUTH_SCRIPT: &str = "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub const DEFAULT_ACCOUNT_CODE: &str = "
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::create_note
    export.::miden::contracts::wallets::basic::move_asset_to_note
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

// HELPER FUNCTIONS
// ================================================================================================

pub fn get_account_with_default_account_code(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let account_code_src = DEFAULT_ACCOUNT_CODE;
    let assembler = TransactionKernel::assembler();

    let account_code = AccountCode::compile(account_code_src, assembler).unwrap();
    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, public_key)], BTreeMap::new()).unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

pub fn get_new_pk_and_authenticator() -> (Word, Rc<BasicAuthenticator<StdRng>>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Rc::new(authenticator))
}

pub fn write_bench_results_to_json(
    path: &Path,
    tx_benchmarks: Vec<(Benchmark, TransactionMeasurements)>,
) -> Result<(), String> {
    // convert benchmark file internals to the JSON Value
    let benchmark_file = read_to_string(path).map_err(|e| e.to_string())?;
    let mut benchmark_json: Value = from_str(&benchmark_file).map_err(|e| e.to_string())?;

    // fill becnhmarks JSON with results of each benchmark
    for (bench_type, tx_progress) in tx_benchmarks {
        let tx_benchmark_json = serde_json::to_value(tx_progress).map_err(|e| e.to_string())?;

        benchmark_json[bench_type.to_string()] = tx_benchmark_json;
    }

    // write the becnhmarks JSON to the results file
    write(
        path,
        to_string_pretty(&benchmark_json).expect("failed to convert json to String"),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
