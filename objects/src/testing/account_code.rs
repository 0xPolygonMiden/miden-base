use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0x66f458f3fe6f827da6a14008a063729e7197e419f2511185c8d03709d152eca4",
    "0x41a482ae5f9fbd220f8bbdcb2ac1ae457d5c4a0ab2f964d9ac427933eef4c4cb",
    "0x9350d01518a45b0b131cc54f85018f2e2ab42b785134507240f7c2eea20c41a6",
    "0x066a992d29e645d266c79e0e80da40d973d99b695b0d1c56206576d290aa0b43",
    "0x6ded055b607f2b6fae52501068ae014c6ee2d671b815128f4af9943f967ae238",
    "0x8e24b9e9ab4198a2818634d991004c8219dcbe6b6cf7e04a38dddfff5a116d72",
    "0x9d30c9a2ef62e0d5a18a7494ee6f4fd1e6a7f02adf7f51ddecc6793cf6e98389",
    "0xfb4cf6929a4ad3af2a3e075e04ed2b4101f0007be3616c5b18ccb93c36ca903d",
    "0x73b20b6e5b268587e65ed5437835a5dde8b2282002d48af00a2a46a6f10f2f7b",
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
