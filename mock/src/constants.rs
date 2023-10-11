use miden_objects::{
    accounts::{AccountId, StorageItem},
    assets::{Asset, NonFungibleAsset, NonFungibleAssetDetails},
    Felt, FieldElement,
};

pub const ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: [u64; 4] = [
    2394600421062944048,
    5272776192057260276,
    17694074059714973576,
    18321641348679662936,
];
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 2731586360824823714;
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
    "0x108b8fae888827e2c106af8567c308da9f8d8f9bb5c88a24c0ca6e394d73ffdb";
pub const ACCOUNT_PROCEDURE_SET_ITEM_MAST_ROOT: &str =
    "0xaa7d21ef60871f591cbb16b85907c6674c0a834740deab336c348c95269a747f";

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
