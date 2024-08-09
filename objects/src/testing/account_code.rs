use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 13] = [
    "0x8afef26974e7fdb12d7e2af49e92f68b09c504e5c530c1283711653f242fd350",
    "0x9a51f741356a6c27dd116c590ca116463ff41d46fddc890c0aa6c2dc0a5c86a9",
    "0xe3c24a1109379344874ac5dec91a6311e5563d0194ded29b44ed71535e78b34a",
    "0x10271f1549303e22f9562bfdb73da948407a276ea0ec470deb7ce6793c316562",
    "0x28c514e509fc044a2ea6cddbab0abf2b5fa589d5c91978ae9c935ab40e6ec402",
    "0xa61cdf8c75943d293ffcfca73ea07a6639dad1820d64586a2a292bb9f80a4296",
    "0x6877f03ef52e490f7c9e41b297fb79bb78075ff28c6e018aaa1ee30f73e7ea4b",
    "0x24e0a1587d4d1ddff74313518f5187f6042ffbe8f2ddc97d367a5c3da4b17d82",
    "0x8afef26974e7fdb12d7e2af49e92f68b09c504e5c530c1283711653f242fd350",
    "0x1e7fdec663be004a894d024a9354e5721f0c1b439ecce73a40e1689cf5379511",
    "0xcd34115714cdcda24f1d6968cbfb67b8b51c1751a2e25e9d6b4e18c35323e5ba",
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
];
pub const ACCOUNT_SEND_ASSET_MAST_ROOT: &str = MASTS[1];
pub const ACCOUNT_INCR_NONCE_MAST_ROOT: &str = MASTS[4];
pub const ACCOUNT_SET_ITEM_MAST_ROOT: &str = MASTS[5];
pub const ACCOUNT_SET_MAP_ITEM_MAST_ROOT: &str = MASTS[6];
pub const ACCOUNT_SET_CODE_MAST_ROOT: &str = MASTS[7];
pub const ACCOUNT_CREATE_NOTE_MAST_ROOT: &str = MASTS[8];
pub const ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT: &str = MASTS[9];
pub const ACCOUNT_REMOVE_ASSET_MAST_ROOT: &str = MASTS[10];
pub const ACCOUNT_ACCOUNT_PROCEDURE_1_MAST_ROOT: &str = MASTS[11];
pub const ACCOUNT_ACCOUNT_PROCEDURE_2_MAST_ROOT: &str = MASTS[12];

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
        export.wallet::cteate_note
        # acct proc 3
        export.wallet::move_asset_to_note

        # acct proc 4
        export.incr_nonce
            push.0 swap
            # => [value, 0]

            exec.account::incr_nonce
            # => [0]
        end

        # acct proc 5
        export.set_item
            exec.account::set_item
            # => [R', V, 0, 0, 0]

            movup.8 drop movup.8 drop movup.8 drop
            # => [R', V]
        end

        # acct proc 6
        export.set_map_item
            exec.account::set_map_item
            # => [R', V, 0, 0, 0]

            movup.8 drop movup.8 drop movup.8 drop
            # => [R', V]
        end

        # acct proc 7
        export.set_code
            padw swapw
            # => [CODE_COMMITMENT, 0, 0, 0, 0]

            exec.account::set_code
            # => [0, 0, 0, 0]
        end

        # acct proc 8
        export.create_note
            exec.tx::create_note
            # => [note_idx]
        end

        # acct proc 9
        export.add_asset_to_note
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]
        end

        # acct proc 10
        export.remove_asset
            exec.account::remove_asset
            # => [ASSET]
        end

        # acct proc 11
        export.account_procedure_1
            push.1.2
            add
        end

        # acct proc 12
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
            code.procedures()[11].mast_root().to_hex(),
            code.procedures()[12].mast_root().to_hex(),
        ];
        assert!(current == MASTS, "const MASTS: [&str; 13] = {:?};", current);

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
