use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0xbb58a032a1c1989079dcc73c279d69dcdf41dd7ee923d99dc3f86011663ec167",
    "0x49c463b2b0888fc21973956b5bf93a62b09ad580169a392310e8b06fb16e7063",
    "0x15cc8a22ff11a964556d2c0be7c34b8a6c8823aeae8b3fb85adf0debdbf8299c",
    "0x4ce781587b60e18c13251445da6a24cf758bbb83726019f941d0555ef65a8b1b",
    "0xff9b31930a10a0725f0e950f6f59c40e96799e67704103dc86ad04ce32526998",
    "0x88a3caaf8117785b056dddcebad8f283cdc3f05e6b518ee841c2e7515df38ee1",
    "0x686a897c14304aeee4b3047b7a217a3209d14f6530bd3bdf33fcd5c314c713e5",
    "0xf484a84dad7f82e8eb1d5190b43243d02d9508437ff97522e14ebf9899758faa",
    "0xf17acfc7d1eff3ecadd7a17b6d91ff01af638aa9439d6c8603c55648328702ae",
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
];

pub const ACCOUNT_RECEIVE_ASSET_MAST_ROOT: &str = MASTS[0];
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

pub const CODE: &str = "
    export.foo
        push.1 push.2 mul
    end

    export.bar
        push.1 push.2 add
    end
";

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

impl AccountCode {
    /// Creates a mock [AccountCode] that exposes wallet interface
    pub fn mock_wallet(assembler: &Assembler) -> AccountCode {
        let account_code = "\
        use.miden::account
        use.miden::tx
        use.miden::contracts::wallets::basic->wallet

        #######################################  IMPORTANT  #######################################
        #                                                                                         #
        #            Only use locally defined procedures below instead of re-exports              #
        #                                see miden-vm/#1376                                       #
        #                                                                                         #
        ###########################################################################################

        # acct proc 0
        export.receive_asset
            exec.wallet::receive_asset
        end

        # acct proc 1
        export.send_asset.1
            exec.wallet::send_asset
        end

        # acct proc 2
        export.incr_nonce
            exec.wallet::incr_nonce
        end

        # acct proc 3
        export.set_item
            exec.account::set_item
        end

        # acct proc 4
        export.set_map_item
            exec.account::set_map_item
        end

        # acct proc 5
        export.set_code
            exec.wallet::set_code
        end

        # acct proc 6
        export.create_note
            exec.tx::create_note
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
            code.procedures()[0].to_hex(),
            code.procedures()[1].to_hex(),
            code.procedures()[2].to_hex(),
            code.procedures()[3].to_hex(),
            code.procedures()[4].to_hex(),
            code.procedures()[5].to_hex(),
            code.procedures()[6].to_hex(),
            code.procedures()[7].to_hex(),
            code.procedures()[8].to_hex(),
            code.procedures()[9].to_hex(),
            code.procedures()[10].to_hex(),
        ];
        assert!(current == MASTS, "const MASTS: [&str; 11] = {:?};", current);

        code
    }

    /// Creates a mock [AccountCode] with default assembler and mock code
    pub fn mock() -> AccountCode {
        let mut module = ModuleAst::parse(CODE).unwrap();
        // clears are needed since they're not serialized for account code
        module.clear_imports();
        module.clear_locations();
        AccountCode::new(module, &Assembler::default()).unwrap()
    }
}
