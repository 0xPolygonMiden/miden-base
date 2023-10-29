use miden_objects::{
    accounts::{AccountId, StorageItem},
    assets::{Asset, NonFungibleAsset, NonFungibleAssetDetails},
    Felt, FieldElement,
};
pub const ACCOUNT_SEED_FUNGIBLE_FAUCET_INVALID_INITIAL_BALANCE: [u64; 4] = [
    6145570607758938301,
    10282880913774515349,
    6426741982522617727,
    6405580465831572650,
];
pub const ACCOUT_ID_FUNGIBLE_FAUCET_INVALID_INITIAL_BALANCE: u64 = 13422896179450902740;

pub const ACCOUNT_SEED_FUNGIBLE_FAUCET_VALID_INITIAL_BALANCE: [u64; 4] = [
    9588068054595421519,
    16811868114829517529,
    5373761197620064059,
    7563481159681753098,
];
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_VALID_INITIAL_BALANCE: u64 = 12328054752197811524;

pub const ACCOUNT_SEED_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: [u64; 4] = [
    11360754003635610262,
    1645235213184378605,
    12058267732908752911,
    223114579406030011,
];
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT: u64 = 16443721535164139279;

pub const ACCOUNT_SEED_NON_FUNGIBLE_FAUCET_VALID_RESERVED_SLOT: [u64; 4] = [
    404699601172309312,
    12905832155459206783,
    9802402797413803903,
    13510058645612144083,
];
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_VALID_RESERVED_SLOT: u64 = 17909431462585405459;

pub const ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: [u64; 4] = [
    10873503761844905100,
    14565999216237198843,
    1403914022137382820,
    12586397471557782933,
];
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 2817606756080467693;

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
