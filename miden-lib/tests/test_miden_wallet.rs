
fn test_add_asset_via_wallet() {
    for storage_item in [STORAGE_ITEM_0, STORAGE_ITEM_1] {
        let (account, block_header, chain, notes) = mock_inputs(AccountStatus::Existing);

        let code = format!(
            "
        use.miden::sat::miden_wallet

        begin
            # prepare the transaction
            exec.miden_wallet::add_asset

            # push the account storage item index
            push.{item_index}

            # get the item
            exec.account::get_item

            # assert the item value is correct
            push.{item_value} assert_eqw
        end
        ",
            item_index = storage_item.0,
            item_value = prepare_word(&storage_item.1)
        );

        let transaction =
            prepare_transaction(account, None, block_header, chain, notes, &code, "", None, None);

        let _process = run_tx(
            transaction.tx_program().clone(),
            StackInputs::from(transaction.stack_inputs()),
            MemAdviceProvider::from(transaction.advice_provider_inputs()),
        )
        .unwrap();
    }
}
