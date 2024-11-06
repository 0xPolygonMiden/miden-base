extern crate alloc;
pub use alloc::{collections::BTreeMap, string::String};
use std::sync::Arc;

use miden_lib::accounts::{auth::RpoFalcon512, wallets::BasicWallet};
use miden_objects::{
    accounts::{Account, AccountId, AuthSecretKey},
    assets::{Asset, AssetVault},
    crypto::dsa::rpo_falcon512::{PublicKey, SecretKey},
    transaction::TransactionMeasurements,
    Felt, Word,
};
use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
use rand::rngs::StdRng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::{from_str, to_string_pretty, Value};

use super::{read_to_string, write, Benchmark, Path};

// CONSTANTS
// ================================================================================================

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0x200000000000001f; // 2305843009213693983
pub const ACCOUNT_ID_SENDER: u64 = 0x800000000000001f; // 9223372036854775839
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0x900000000000003f; // 10376293541461622847

pub const DEFAULT_AUTH_SCRIPT: &str = "
    begin
        call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
    end
";

// MEASUREMENTS PRINTER
// ================================================================================================

#[derive(Debug, Clone, Serialize)]
pub struct MeasurementsPrinter {
    prologue: usize,
    notes_processing: usize,
    note_execution: BTreeMap<String, usize>,
    tx_script_processing: usize,
    epilogue: usize,
}

impl From<TransactionMeasurements> for MeasurementsPrinter {
    fn from(value: TransactionMeasurements) -> Self {
        let note_execution_map =
            value.note_execution.iter().map(|(id, len)| (id.to_hex(), *len)).collect();

        MeasurementsPrinter {
            prologue: value.prologue,
            notes_processing: value.notes_processing,
            note_execution: note_execution_map,
            tx_script_processing: value.tx_script_processing,
            epilogue: value.epilogue,
        }
    }
}

// HELPER FUNCTIONS
// ================================================================================================

pub fn get_account_with_basic_authenticated_wallet(
    account_id: AccountId,
    public_key: Word,
    assets: Option<Asset>,
) -> Account {
    let (account_code, account_storage) = Account::initialize_from_components(
        account_id.account_type(),
        &[BasicWallet.into(), RpoFalcon512::new(PublicKey::new(public_key)).into()],
    )
    .unwrap();

    let account_vault = match assets {
        Some(asset) => AssetVault::new(&[asset]).unwrap(),
        None => AssetVault::new(&[]).unwrap(),
    };

    Account::from_parts(account_id, account_vault, account_storage, account_code, Felt::new(1))
}

pub fn get_new_pk_and_authenticator() -> (Word, Arc<dyn TransactionAuthenticator>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Arc::new(authenticator) as Arc<dyn TransactionAuthenticator>)
}

pub fn write_bench_results_to_json(
    path: &Path,
    tx_benchmarks: Vec<(Benchmark, MeasurementsPrinter)>,
) -> Result<(), String> {
    // convert benchmark file internals to the JSON Value
    let benchmark_file = read_to_string(path).map_err(|e| e.to_string())?;
    let mut benchmark_json: Value = from_str(&benchmark_file).map_err(|e| e.to_string())?;

    // fill benchmarks JSON with results of each benchmark
    for (bench_type, tx_progress) in tx_benchmarks {
        let tx_benchmark_json = serde_json::to_value(tx_progress).map_err(|e| e.to_string())?;

        benchmark_json[bench_type.to_string()] = tx_benchmark_json;
    }

    // write the benchmarks JSON to the results file
    write(
        path,
        to_string_pretty(&benchmark_json).expect("failed to convert json to String"),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
