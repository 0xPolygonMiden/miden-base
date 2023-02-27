use super::{AccountId, AccountType};

const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;
const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN: u64 = 0b0001101110 << 54;
const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;
const ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN: u64 = 0b1101100110 << 54;

#[test]
fn test_account_tag_identifiers() {
    let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN)
        .expect("Valid account ID");
    assert!(account_id.is_regular_account());
    assert_eq!(account_id.account_type(), AccountType::RegularAccountImmutableCode);
    assert!(account_id.is_on_chain());

    let account_id = AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN)
        .expect("Valid account ID");
    assert!(account_id.is_regular_account());
    assert_eq!(account_id.account_type(), AccountType::RegularAccountUpdatableCode);
    assert!(!account_id.is_on_chain());

    let account_id =
        AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).expect("Valid account ID");
    assert!(account_id.is_faucet());
    assert_eq!(account_id.account_type(), AccountType::FungibleFaucet);
    assert!(account_id.is_on_chain());

    let account_id =
        AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN).expect("Valid account ID");
    assert!(account_id.is_faucet());
    assert_eq!(account_id.account_type(), AccountType::NonFungibleFaucet);
    assert!(!account_id.is_on_chain());
}
