use miden_objects::{
    accounts::{AccountId, StorageItem},
    assets::{Asset, NonFungibleAsset, NonFungibleAssetDetails},
    Felt, FieldElement,
};

pub const ACCOUNT_SEED_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: [u64; 4] = [
    6131122028735692579,
    9917745298979862884,
    5350803933723696266,
    15070968458072451245,
];
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 3606314465504722753;
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

pub const CONSUMED_NOTE_1_AMOUNT: u64 = 100;
pub const CONSUMED_NOTE_2_AMOUNT: u64 = 200;
pub const CONSUMED_NOTE_3_AMOUNT: u64 = 300;

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
