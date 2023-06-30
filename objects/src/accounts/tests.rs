use super::{AccountId, AccountType, Felt, Word, EMPTY_WORD};
use crate::{
    mock::{
        assembler, mock_account, CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH, STORAGE_ITEM_0,
        STORAGE_ITEM_1,
    },
    AccountCode,
};
use assembly::ast::ModuleAst;
use crypto::utils::collections::{ApplyDiff, Diff};
use miden_core::crypto::merkle::NodeIndex;

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

#[test]
fn test_account_diff() {
    let mut init_account = mock_account(None, None, &mut assembler());
    let updated_account_code = ModuleAst::parse(
        "\
    export.account_procedure_1
        push.5.6
        add
    end

    export.account_procedure_2
        push.11.12
        sub
    end
    ",
    )
    .unwrap();
    let updated_account_code =
        AccountCode::new(init_account.id, updated_account_code, &mut assembler()).unwrap();
    let mut final_account = mock_account(None, Some(updated_account_code), &mut assembler());

    // ACCOUNT STORAGE CHANGES
    // --------------------------------------------------------------------------------------------
    // add a new storage slot
    const NEW_STORAGE_SLOT_IDX: u8 = 120;
    const NEW_STORAGE_SLOT_VALUE: Word =
        [Felt::new(100), Felt::new(101), Felt::new(102), Felt::new(103)];
    final_account
        .storage_mut()
        .set_item(NEW_STORAGE_SLOT_IDX, NEW_STORAGE_SLOT_VALUE);

    // delete a storage slot
    final_account.storage_mut().set_item(STORAGE_ITEM_0.0, EMPTY_WORD);

    // update a storage slot
    const UPDATED_STORAGE_SLOT_VALUE: Word =
        [Felt::new(200), Felt::new(201), Felt::new(202), Felt::new(203)];
    final_account
        .storage_mut()
        .set_item(STORAGE_ITEM_1.0, UPDATED_STORAGE_SLOT_VALUE);

    // add a new child storage item
    const NEW_CHILD_ITEM_VALUE: Word =
        [Felt::new(300), Felt::new(301), Felt::new(302), Felt::new(303)];
    const NEW_CHILD_ITEM_INDEX: u64 = 200;
    let node_index = NodeIndex::new(CHILD_SMT_DEPTH, NEW_CHILD_ITEM_INDEX).unwrap();
    let new_root = final_account
        .storage_mut()
        .set_store_node(CHILD_ROOT_PARENT_LEAF_INDEX, node_index, NEW_CHILD_ITEM_VALUE.into())
        .unwrap();

    let child_item = final_account.storage().store().get_node(new_root, node_index).unwrap();

    assert_eq!(child_item, NEW_CHILD_ITEM_VALUE.into());

    // ACCOUNT NONCE CHANGE
    // --------------------------------------------------------------------------------------------
    final_account.set_nonce(Felt::new(100)).unwrap();

    // ACCOUNT VAULT CHANGES
    // --------------------------------------------------------------------------------------------
    // TODO: Add vault changes

    // ACCOUNT DELTA
    // --------------------------------------------------------------------------------------------
    let account_diff = init_account.diff(&final_account);

    // ASSERTIONS
    // --------------------------------------------------------------------------------------------
    // Assert updates and cleared slots work as expected
    init_account.apply(account_diff);
    assert_eq!(init_account.storage.root(), final_account.storage.root());
    assert_eq!(init_account.nonce(), final_account.nonce());
    assert_eq!(init_account.vault.commitment(), final_account.vault().commitment());
    assert_eq!(init_account.code.root(), final_account.code.root());

    // assert new storage slot is reflected
    let node = init_account.storage.get_item(NEW_STORAGE_SLOT_IDX);
    assert_eq!(node, NEW_STORAGE_SLOT_VALUE.into());

    // assert deleted storage slot is reflected
    let node = init_account.storage.get_item(STORAGE_ITEM_0.0);
    assert_eq!(node, EMPTY_WORD.into());

    // assert updated storage slot is reflected
    let node = init_account.storage.get_item(STORAGE_ITEM_1.0);
    assert_eq!(node, UPDATED_STORAGE_SLOT_VALUE.into());

    // Assert new child storage item is added
    let node = init_account.storage.store().get_node(new_root, node_index).unwrap();
    assert_eq!(node, NEW_CHILD_ITEM_VALUE.into());

    // Assert that the account code is the same
    assert_eq!(init_account.code, final_account.code);
}
