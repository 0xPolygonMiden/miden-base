use crate::accounts::{AccountId, AccountStorageMode, AccountType};

// CONSTANTS
// --------------------------------------------------------------------------------------------

// REGULAR ACCOUNTS - OFF-CHAIN
pub const ACCOUNT_ID_SENDER: u128 = account_id::<true>(
    AccountType::RegularAccountImmutableCode,
    AccountStorageMode::Private,
    0xfabb_ccde,
);
pub const ACCOUNT_ID_OFF_CHAIN_SENDER: u128 = account_id::<true>(
    AccountType::RegularAccountImmutableCode,
    AccountStorageMode::Private,
    0xbfcc_dcee,
);
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u128 = account_id::<true>(
    AccountType::RegularAccountUpdatableCode,
    AccountStorageMode::Private,
    0xccdd_eeff,
);
// REGULAR ACCOUNTS - ON-CHAIN
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u128 = account_id::<true>(
    AccountType::RegularAccountImmutableCode,
    AccountStorageMode::Public,
    0xaabb_ccdd,
);
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN_2: u128 = account_id::<true>(
    AccountType::RegularAccountImmutableCode,
    AccountStorageMode::Public,
    0xbbcc_ddee,
);
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u128 = account_id::<true>(
    AccountType::RegularAccountUpdatableCode,
    AccountStorageMode::Public,
    0xacdd_eefc,
);
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2: u128 = account_id::<true>(
    AccountType::RegularAccountUpdatableCode,
    AccountStorageMode::Public,
    0xeeff_ccdd,
);

// These faucet IDs all have a unique first and second felt. This is to ensure that when they
// are used to issue an asset they don't cause us to run into the "multiple leaf" case when
// calling std::collections::smt::{set,get} which doesn't support the "multiple leaf" case at
// this time.

// FUNGIBLE TOKENS - OFF-CHAIN
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
    account_id::<true>(AccountType::FungibleFaucet, AccountStorageMode::Private, 0xfabb_cddd);
// FUNGIBLE TOKENS - ON-CHAIN
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
    account_id::<true>(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xaabc_bcde);
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
    account_id::<true>(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xbaca_ddef);
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2: u128 =
    account_id::<true>(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xccdb_eefa);
pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3: u128 =
    account_id::<true>(AccountType::FungibleFaucet, AccountStorageMode::Public, 0xeeff_cc99);

// NON-FUNGIBLE TOKENS - OFF-CHAIN
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u128 =
    account_id::<true>(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0xaabc_ccde);
// NON-FUNGIBLE TOKENS - ON-CHAIN
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN: u128 =
    account_id::<true>(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0xbcca_ddef);
pub const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN_1: u128 =
    account_id::<true>(AccountType::NonFungibleFaucet, AccountStorageMode::Public, 0xccdf_eefa);

// TEST ACCOUNT IDs WITH CERTAIN PROPERTIES
/// The Account Id with the maximum possible one bits.
pub const ACCOUNT_ID_MAX_ONES: u128 =
    account_id::<false>(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0)
        | 0x7fff_ffff_ffff_ff00_7fff_ffff_ffff_ff00;
/// The Account Id with the maximum possible zero bits.
pub const ACCOUNT_ID_MAX_ZEROES: u128 =
    account_id::<true>(AccountType::NonFungibleFaucet, AccountStorageMode::Private, 0x001f_0000);

// UTILITIES
// --------------------------------------------------------------------------------------------

/// Produces a valid account ID with the given account type and storage mode.
///
/// - Version is set to 0.
/// - Anchor epoch is set to 0.
///
/// Finally, distributes the given `random` value over the ID to produce non-trivial values for
/// testing. This is easiest explained with an example. Suppose `random` is `0xaabb_ccdd`,
/// then the layout of the generated ID will be:
///
/// ```text
/// 1st felt: [zero byte | 0xaa | 4 zero bytes | 0xbb | metadata byte]
/// 2nd felt: [2 zero bytes (epoch) | 0xcc | 3 zero bytes | 0xdd | zero byte]
/// ```
pub const fn account_id<const CHECK_MIN_ONES: bool>(
    account_type: AccountType,
    storage_mode: AccountStorageMode,
    random: u32,
) -> u128 {
    let mut first_felt: u64 = 0;

    first_felt |= (account_type as u64) << AccountId::TYPE_SHIFT;
    first_felt |= (storage_mode as u64) << AccountId::STORAGE_MODE_SHIFT;

    // Produce non-trivial IDs by distributing the random value.
    let random_1st_felt_upper = random & 0xff00_0000;
    let random_1st_felt_lower = random & 0x00ff_0000;
    let random_2nd_felt_upper = random & 0x0000_ff00;
    let random_2nd_felt_lower = random & 0x0000_00ff;

    // Shift the random part of the ID to start at the most significant end.
    first_felt |= (random_1st_felt_upper as u64) << 24;
    first_felt |= (random_1st_felt_lower as u64) >> 8;

    let mut id = (first_felt as u128) << 64;

    id |= (random_2nd_felt_upper as u128) << 32;
    id |= (random_2nd_felt_lower as u128) << 8;

    id
}
