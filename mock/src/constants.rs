use miden_objects::{
    accounts::{AccountId, StorageItem},
    assets::{Asset, NonFungibleAsset, NonFungibleAssetDetails},
    Felt, FieldElement,
};
pub const ACCOUNT_SEED_FUNGIBLE_FAUCET_INVALID_INITIAL_BALANCE: [u64; 4] = [
    5342472004481420725,
    15139745540144612214,
    7175220148257528278,
    12185026252347356330,
];
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_INVALID_INITIAL_BALANCE: u64 = 11808006999189383835;

pub const ACCOUNT_SEED_FUNGIBLE_FAUCET_VALID_INITIAL_BALANCE: [u64; 4] = [
    2389651479266964250,
    12570482780864789472,
    3827181395997035738,
    15484731259405424484,
];
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_VALID_INITIAL_BALANCE: u64 = 13650031744811031251;

pub const ACCOUNT_SEED_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: [u64; 4] = [
    6127036790776509692,
    7202481422049357184,
    4843524022082280619,
    10803188976143115680,
];
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: u64 = 16992539250668925014;

pub const ACCOUNT_SEED_NON_FUNGIBLE_FAUCET_VALID_RESERVED_SLOT: [u64; 4] = [
    1480622438474629078,
    7551952530545164022,
    14268335233603228254,
    322360296984864845,
];
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_VALID_RESERVED_SLOT: u64 = 17114871292473597866;

pub const ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: [u64; 4] = [
    4160082958698397031,
    5735601916245113230,
    4289496549565608242,
    16940108222232119545,
];
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3238098370154045919;

pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010111100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1110011100 << 54;
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 = 0b1110011101 << 54;
pub const FUNGIBLE_ASSET_AMOUNT: u64 = 100;
pub const FUNGIBLE_FAUCET_INITIAL_BALANCE: u64 = 50000;

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u64 =
    0b1010010001111111010110100011011110101011010001101111110110111100u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u64 =
    0b1010000101101010101101000110111101010110100011011110100011011101u64;
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u64 =
    0b1010011001011010101101000110111101010110100011011101000110111100u64;

// Default account code
pub const DEFAULT_ACCOUNT_CODE: &str = "
    use.miden::wallets::basic->basic_wallet
    use.miden::eoa::basic->basic_eoa

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic_eoa::auth_tx_rpo_falcon512
";

pub const ACCOUNT_PROCEDURE_INCR_NONCE_MAST_ROOT: &str =
    "0xd765111e22479256e87a57eaf3a27479d19cc876c9a715ee6c262e0a0d47a2ac";
pub const ACCOUNT_PROCEDURE_SET_CODE_MAST_ROOT: &str =
    "0x9d221abcc386973775499406d126764cdf4530ccf8084e27091f7e9f28177bbe";
pub const ACCOUNT_PROCEDURE_SET_ITEM_MAST_ROOT: &str =
    "0x49935297f029f8b229fe86c6c47b9d291d063b8558fe90319128fb60dbda3d1b";

pub const CONSUMED_ASSET_1_AMOUNT: u64 = 100;
pub const CONSUMED_ASSET_2_AMOUNT: u64 = 200;
pub const CONSUMED_ASSET_3_AMOUNT: u64 = 300;

pub const NON_FUNGIBLE_ASSET_DATA: [u8; 4] = [1, 2, 3, 4];
pub const NON_FUNGIBLE_ASSET_DATA_2: [u8; 4] = [5, 6, 7, 8];

pub const NONCE: Felt = Felt::ZERO;

pub const STORAGE_INDEX_0: u8 = 20;
pub const STORAGE_VALUE_0: [Felt; 4] = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
pub const STORAGE_INDEX_1: u8 = 30;
pub const STORAGE_VALUE_1: [Felt; 4] = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
pub const STORAGE_ITEM_0: StorageItem = (STORAGE_INDEX_0, STORAGE_VALUE_0);
pub const STORAGE_ITEM_1: StorageItem = (STORAGE_INDEX_1, STORAGE_VALUE_1);

pub const CHILD_ROOT_PARENT_LEAF_INDEX: u8 = 10;
pub const CHILD_SMT_DEPTH: u8 = 64;
pub const CHILD_STORAGE_INDEX_0: u64 = 40;
pub const CHILD_STORAGE_VALUE_0: [Felt; 4] =
    [Felt::new(11), Felt::new(12), Felt::new(13), Felt::new(14)];

pub fn non_fungible_asset(account_id: u64) -> Asset {
    let non_fungible_asset_details = NonFungibleAssetDetails::new(
        AccountId::try_from(account_id).unwrap(),
        NON_FUNGIBLE_ASSET_DATA.to_vec(),
    )
    .unwrap();
    let non_fungible_asset = NonFungibleAsset::new(&non_fungible_asset_details).unwrap();
    Asset::NonFungible(non_fungible_asset)
}

pub fn non_fungible_asset_2(account_id: u64) -> Asset {
    let non_fungible_asset_2_details: NonFungibleAssetDetails = NonFungibleAssetDetails::new(
        AccountId::try_from(account_id).unwrap(),
        NON_FUNGIBLE_ASSET_DATA_2.to_vec(),
    )
    .unwrap();
    let non_fungible_asset_2: NonFungibleAsset =
        NonFungibleAsset::new(&non_fungible_asset_2_details).unwrap();
    Asset::NonFungible(non_fungible_asset_2)
}
