use super::{
    Account, AccountId, Asset, Digest, ExecutedTransaction, Felt, FieldElement, FungibleAsset,
    Note, TransactionInputs, Word,
};

// MOCK DATA
// ================================================================================================
pub const ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN: u64 = 0b0110011011u64 << 54;
pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;

const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

pub const NONCE: Felt = Felt::ZERO;

pub fn mock_inputs() -> TransactionInputs {
    // Create an account
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let account = Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ZERO).unwrap();

    // Block reference
    let block_ref: Digest =
        Digest::new([Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)]);

    // Consumed notes
    let consumed_notes = mock_consumed_notes();

    // Transaction inputs
    TransactionInputs::new(account, block_ref, consumed_notes, None)
}

pub fn mock_executed_tx() -> ExecutedTransaction {
    // AccountId
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();

    // Initial Account
    let initial_account =
        Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ZERO).unwrap();

    // Finial Account (nonce incremented by 1)
    let final_account =
        Account::new(account_id, &[], "proc.test_proc push.1 end", Felt::ONE).unwrap();

    // Consumed notes
    let consumed_notes = mock_consumed_notes();

    // Created notes
    let created_notes = mock_created_notes();

    // Block reference
    let block_ref: Digest =
        Digest::new([Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)]);

    // Executed Transaction
    ExecutedTransaction::new(
        initial_account,
        final_account,
        consumed_notes,
        created_notes,
        None,
        block_ref,
    )
}

fn mock_consumed_notes() -> Vec<Note> {
    // Note Assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 200).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 300).unwrap().into();

    // Sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_1 = Note::new(
        "begin push.1 end",
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_2 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
    )
    .unwrap();

    vec![note_1, note_2]
}

fn mock_created_notes() -> Vec<Note> {
    // Note assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 100).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 100).unwrap().into();

    // sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // Created Notes
    const SERIAL_NUM_1: Word = [Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)];
    let note_1 = Note::new(
        "begin push.1 end",
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let note_2 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
    )
    .unwrap();

    const SERIAL_NUM_3: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let note_3 = Note::new(
        "begin push.1 end",
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_3,
        sender,
        Felt::ZERO,
    )
    .unwrap();

    vec![note_1, note_2, note_3]
}
