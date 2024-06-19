use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0xbb58a032a1c1989079dcc73c279d69dcdf41dd7ee923d99dc3f86011663ec167",
    "0x549d264f00f1a6e90d47284e99eab6d0f93a3d41bb5324743607b6902978a809",
    "0x704ed1af80a3dae74cd4aabeb4c217924813c42334c2695a74e2702af80a4a35",
    "0xa27f4acf44ab50969468ea3fccbaae3893bd2117d2e0a60b7440df4ddb3a4585",
    "0x646ab6d0a53288f01083943116d01f216e77adfe21a495ae8d4670b4be40facf",
    "0x73c14f65d2bab6f52eafc4397e104b3ab22a470f6b5cbc86d4aa4d3978c8b7d4",
    "0x55036198d82d2af653935226c644427162f12e2a2c6b3baf007c9c6f47462872",
    "0xf484a84dad7f82e8eb1d5190b43243d02d9508437ff97522e14ebf9899758faa",
    "0xf17acfc7d1eff3ecadd7a17b6d91ff01af638aa9439d6c8603c55648328702ae",
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
];
pub const ACCOUNT_SEND_ASSET_MAST_ROOT: &str = MASTS[1];
pub const ACCOUNT_INCR_NONCE_MAST_ROOT: &str = MASTS[2];
pub const ACCOUNT_SET_ITEM_MAST_ROOT: &str = MASTS[3];
pub const ACCOUNT_SET_MAP_ITEM_MAST_ROOT: &str = MASTS[4];
pub const ACCOUNT_SET_CODE_MAST_ROOT: &str = MASTS[5];
pub const ACCOUNT_CREATE_NOTE_MAST_ROOT: &str = MASTS[6];
pub const ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT: &str = MASTS[7];
pub const ACCOUNT_REMOVE_ASSET_MAST_ROOT: &str = MASTS[8];
pub const ACCOUNT_ACCOUNT_PROCEDURE_1_MAST_ROOT: &str = MASTS[9];
pub const ACCOUNT_ACCOUNT_PROCEDURE_2_MAST_ROOT: &str = MASTS[10];

// ACCOUNT ASSEMBLY CODE
// ================================================================================================

pub const DEFAULT_ACCOUNT_CODE: &str = "
    use.miden::contracts::wallets::basic->basic_wallet
    use.miden::contracts::auth::basic->basic_eoa

    export.basic_wallet::receive_asset
    export.basic_wallet::send_asset
    export.basic_eoa::auth_tx_rpo_falcon512
";

pub const DEFAULT_AUTH_SCRIPT: &str = "
    use.miden::contracts::auth::basic->auth_tx

    begin
        call.auth_tx::auth_tx_rpo_falcon512
    end
";

pub fn mock_account_code(assembler: &Assembler) -> AccountCode {
    let account_code = "\
            use.miden::account
            use.miden::tx
            use.miden::contracts::wallets::basic->wallet

            # acct proc 0
            export.wallet::receive_asset
            # acct proc 1
            export.wallet::send_asset

            # acct proc 2
            export.incr_nonce
                push.0 swap
                # => [value, 0]

                exec.account::incr_nonce
                # => [0]
            end

            # acct proc 3
            export.set_item
                exec.account::set_item
                # => [R', V, 0, 0, 0]

                movup.8 drop movup.8 drop movup.8 drop
                # => [R', V]
            end

            # acct proc 4
            export.set_map_item
                exec.account::set_map_item
                # => [R', V, 0, 0, 0]

                movup.8 drop movup.8 drop movup.8 drop
                # => [R', V]
            end

            # acct proc 5
            export.set_code
                padw swapw
                # => [CODE_ROOT, 0, 0, 0, 0]

                exec.account::set_code
                # => [0, 0, 0, 0]
            end

            # acct proc 6
            export.create_note
                exec.tx::create_note
                # => [note_idx]

                swapw dropw swap drop
            end

            # acct proc 7
            export.add_asset_to_note
                exec.tx::add_asset_to_note
                # => [note_idx]

                swap drop swap drop swap drop
            end

            # acct proc 8
            export.remove_asset
                exec.account::remove_asset
                # => [ASSET]
            end

            # acct proc 9
            export.account_procedure_1
                push.1.2
                add
            end

            # acct proc 10
            export.account_procedure_2
                push.2.1
                sub
            end
            ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let code = AccountCode::new(account_module_ast, assembler).unwrap();

    // Ensures the mast root constants match the latest version of the code.
    //
    // The constants will change if the library code changes, and need to be updated so that the
    // tests will work properly. If these asserts fail, copy the value of the code (the left
    // value), into the constants.
    //
    // Comparing all the values together, in case multiple of them change, a single test run will
    // detect it.
    let current = [
        code.procedures()[0].0.to_hex(),
        code.procedures()[1].0.to_hex(),
        code.procedures()[2].0.to_hex(),
        code.procedures()[3].0.to_hex(),
        code.procedures()[4].0.to_hex(),
        code.procedures()[5].0.to_hex(),
        code.procedures()[6].0.to_hex(),
        code.procedures()[7].0.to_hex(),
        code.procedures()[8].0.to_hex(),
        code.procedures()[9].0.to_hex(),
        code.procedures()[10].0.to_hex(),
    ];
    assert!(current == MASTS, "const MASTS: [&str; 11] = {:?};", current);

    code
}

pub const CODE: &str = "
        export.foo
            push.1 push.2 mul
        end

        export.bar
            push.1 push.2 add
        end
    ";

pub fn make_account_code() -> AccountCode {
    let mut module = ModuleAst::parse(CODE).unwrap();
    // clears are needed since they're not serialized for account code
    module.clear_imports();
    module.clear_locations();
    AccountCode::new(module, &Assembler::default()).unwrap()
}
