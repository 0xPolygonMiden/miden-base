use miden_objects::accounts::tests::{
    ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
};
use mock::{
    constants::{non_fungible_asset, FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA},
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};
use vm_processor::{Felt, ProcessState};

use super::{Hasher, Word, ONE};

#[test]
fn test_create_fungible_asset_succeeds() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::FungibleFaucet {
            acct_id: ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push asset amount onto stack
            push.{FUNGIBLE_ASSET_AMOUNT}

            # create fungible asset
            exec.asset::create_fungible_asset
        end
        "
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();

    assert_eq!(
        process.get_stack_word(0),
        Word::from([
            Felt::new(FUNGIBLE_ASSET_AMOUNT),
            Felt::new(0),
            Felt::new(0),
            Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN)
        ])
    );
}

#[test]
fn test_create_non_fungible_asset_succeeds() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.miden::kernels::tx::prologue
        use.miden::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push non-fungible asset data hash onto the stack
            push.{non_fungible_asset_data_hash}
            exec.asset::create_non_fungible_asset
        end
        ",
        non_fungible_asset_data_hash = prepare_word(&Hasher::hash(&NON_FUNGIBLE_ASSET_DATA)),
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();
    assert_eq!(process.get_stack_word(0), Word::from(non_fungible_asset));
}

#[test]
fn test_validate_non_fungible_asset() {
    let (tx_inputs, tx_args) = mock_inputs(
        MockAccountType::NonFungibleFaucet {
            acct_id: ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            nonce: ONE,
            empty_reserved_slot: false,
        },
        AssetPreservationStatus::Preserved,
    );
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);
    let encoded = Word::from(non_fungible_asset);

    let code = format!(
        "
        use.miden::kernels::tx::asset

        begin
            push.{asset} exec.asset::validate_non_fungible_asset
        end
        ",
        asset = prepare_word(&encoded)
    );

    let transaction = prepare_transaction(tx_inputs, tx_args, &code, None);
    let process = run_tx(&transaction).unwrap();
    assert_eq!(process.get_stack_word(0), encoded);
}
