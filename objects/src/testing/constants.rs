use vm_core::Felt;

pub const FUNGIBLE_ASSET_AMOUNT: u64 = 100;
pub const FUNGIBLE_FAUCET_INITIAL_BALANCE: u64 = 50000;

pub const MIN_PROOF_SECURITY_LEVEL: u32 = 96;

pub const CONSUMED_ASSET_1_AMOUNT: u64 = 100;
pub const CONSUMED_ASSET_2_AMOUNT: u64 = 200;
pub const CONSUMED_ASSET_3_AMOUNT: u64 = 300;
pub const CONSUMED_ASSET_4_AMOUNT: u64 = 100;

pub const NON_FUNGIBLE_ASSET_DATA: [u8; 4] = [1, 2, 3, 4];
pub const NON_FUNGIBLE_ASSET_DATA_2: [u8; 4] = [5, 6, 7, 8];

pub const CHILD_ROOT_PARENT_LEAF_INDEX: u8 = 10;
pub const CHILD_SMT_DEPTH: u8 = 64;
pub const CHILD_STORAGE_INDEX_0: u64 = 40;
pub const CHILD_STORAGE_VALUE_0: [Felt; 4] =
    [Felt::new(11), Felt::new(12), Felt::new(13), Felt::new(14)];
