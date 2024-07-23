use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0x87eafa2283a8cbc50278da6ffca1dc4bb8563a08a6ac87c9d73c82ae3ef3535d",
    "0xd95712ead4206b64d2731af690d727ae8a80713d3c4743911acd6875493ddc49",
    "0x4dad12a153b84b140462f607467bf46395b9d8989810cd07c5c2ce69c6a35c19",
    "0x7af217c46a687dcb90f2a12a4ace9af075863614418cdccbeb5aef3146a516ba",
    "0x1aede2c29cb61fb387573a808c874b92fb0a051707e2965b1575baf3748be40a",
    "0x4b77849c1e7a3a60eb8607492b3ce881fa18b376c8e2f6edfb1f944cf52eea12",
    "0x31a0a78bd8fb355402e677bc5312bef480e8a66975f3f3187d9e1c9924226219",
    "0x25f7cd69f2d3687686fcfdb913264ab6d114f77d11e5b21496c96a5731501143",
    "0xc631e2579bb5844bb7a8ae8000b80d00c4144f83b8b0d813159659cb5c88c678",
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
            # => [CODE_COMMITMENT, 0, 0, 0, 0]

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
            code.procedures()[0].mast_root().to_hex(),
            code.procedures()[1].mast_root().to_hex(),
            code.procedures()[2].mast_root().to_hex(),
            code.procedures()[3].mast_root().to_hex(),
            code.procedures()[4].mast_root().to_hex(),
            code.procedures()[5].mast_root().to_hex(),
            code.procedures()[6].mast_root().to_hex(),
            code.procedures()[7].mast_root().to_hex(),
            code.procedures()[8].mast_root().to_hex(),
            code.procedures()[9].mast_root().to_hex(),
            code.procedures()[10].mast_root().to_hex(),
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
