use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0x133ce9f5ed6fc3f5f61d61921d478e314db943969ef1254118c876ee9d15df2b",
    "0x1c6aad8e723f0d2ba06342ac5e4b5bc88fd4e3b3c37a040d6edf423462f0279f",
    "0xb0a1c2908491baccbb53cdf7eb46f53fcf32e6ab41ba79d506bd14fbaa015699",
    "0x8c6365afa57f1c9742622a00ae3b9da13f806157847c0bb91167c14f74632c00",
    "0x8937839c39f7574cfb16a4041892b605e09089fef31373c8b77486e23c1810af",
    "0x3052745e8ef39d0e9ae9658a8c96a3a182268450e1afc350ab5de9936bc3a020",
    "0x76785a9d56a62164dc6d61cba12332175087e43a853d14c9a053cab607d440e9",
    "0x7b0c4bc7d4ccac27e95503a3b04e45ba476134a49e39b34484f46521244a70d8",
    "0x32a1237a3cc0e74739c2206d8179a3795cbb25c68c07064333bb5fd8740ccd95",
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

    /// Creates a mock [AccountCode] with default assembler and mock code
    pub fn mock() -> AccountCode {
        let mut module = ModuleAst::parse(CODE).unwrap();
        // clears are needed since they're not serialized for account code
        module.clear_imports();
        module.clear_locations();
        AccountCode::new(module, &Assembler::default()).unwrap()
    }
}
