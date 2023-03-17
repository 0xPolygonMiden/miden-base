pub use crypto::{
    hash::rpo::{Rpo256 as Hasher, RpoDigest as Digest},
    FieldElement, StarkField,
};
use miden_lib::MidenLib;
pub use miden_objects::{
    assets::{Asset, FungibleAsset},
    notes::Note,
    transaction::TransactionInputs,
    Account, AccountId,
};
pub use processor::{
    math::Felt, AdviceInputs, AdviceProvider, ExecutionError, MemAdviceProvider, Process,
    StackInputs, Word,
};
use std::{env, fs::File, io::Read, path::Path};

pub const TX_KERNEL_DIR: &str = "sat";

// MEMORY OFFSETS
// ================================================================================================

pub mod memory {
    // BOOKKEEPING
    // --------------------------------------------------------------------------------------------
    pub const CURRENT_CONSUMED_NOTE_PTR: u64 = 3;

    // GLOBAL DATA
    // --------------------------------------------------------------------------------------------
    pub const BLK_HASH_PTR: u64 = 10;
    pub const ACCT_ID_PTR: u64 = 11;
    pub const ACCT_HASH_PTR: u64 = 12;
    pub const NULLIFIER_COM_PTR: u64 = 13;

    // ACCOUNT DATA
    // --------------------------------------------------------------------------------------------
    pub const ACCOUNT_DATA_OFFSET: u64 = 100;
    pub const ACCT_NONCE_PTR: u64 = 100;
    pub const ACCT_VAULT_ROOT_PTR: u64 = 101;
    pub const ACCT_STORAGE_ROOT_PTR: u64 = 102;
    pub const ACCT_CODE_ROOT_PTR: u64 = 103;

    // CONSUMED NOTES DATA
    // --------------------------------------------------------------------------------------------
    pub const CONSUMED_NOTE_DATA_OFFSET: u64 = 1000;
}

// MOCK DATA
// ================================================================================================
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;

const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

pub const NONCE: Felt = Felt::ZERO;

pub fn mock_inputs() -> TransactionInputs {
    // Create an account
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let account = Account::new(account_id, &[], "proc.test_proc push.1 end").unwrap();

    // Create some assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 200).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 300).unwrap().into();

    // Create two notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_1 = Note::new(
        "begin push.1 end",
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_1,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_2 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
    )
    .unwrap();

    // Create block reference
    let block_ref: Digest =
        Digest::new([Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)]);

    TransactionInputs::new(account, block_ref, vec![note_1, note_2], None)
}

// TEST BRACE
// ================================================================================================

/// Loads the specified file and append `code` into its end.
pub fn load_file_with_code(imports: &str, code: &str, dir: &str, file: &str) -> String {
    let assembly_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("asm").join(dir).join(file);

    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

/// Inject `code` along side the specified file and run it
pub fn run_within_tx_kernel<A>(
    imports: &str,
    code: &str,
    stack_inputs: StackInputs,
    adv: A,
    dir: &str,
    file: &str,
) -> Process<A>
where
    A: AdviceProvider,
{
    let assembler = assembly::Assembler::default()
        .with_library(&MidenLib::default())
        .expect("failed to load stdlib");

    let code = load_file_with_code(imports, code, dir, file);
    let program = assembler.compile(code).unwrap();

    let mut process = Process::new(program.kernel().clone(), stack_inputs, adv);
    process.execute(&program).unwrap();

    process
}

// TEST HELPERS
// ================================================================================================
pub fn note_data_ptr(note_idx: u64) -> u64 {
    memory::CONSUMED_NOTE_DATA_OFFSET + (1 + note_idx) * 1024
}
