use assembly::{ast::ModuleAst, Assembler};

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 11] = [
    "0x74de7e94e5afc71e608f590c139ac51f446fc694da83f93d968b019d1d2b7306",
    "0xf5f4a93b873d3dc236539d5566d245ac5e5bc6be75fc5af4235a52fe091077ae",
    "0xd765111e22479256e87a57eaf3a27479d19cc876c9a715ee6c262e0a0d47a2ac",
    "0x17b326d5403115afccc0727efa72bd929bfdc7bbf284c7c28a7aadade5d4cc9d",
    "0x6682a0e0f4e49820e5c547f1b60a82cb326a56c972999e36bf6d45459393ac87",
    "0x73c14f65d2bab6f52eafc4397e104b3ab22a470f6b5cbc86d4aa4d3978c8b7d4",
    "0x49fee714925e6b287136494465184a84495cedb35fce3ab3a13f68ad48751596",
    "0xfe4b6f0a485393583f5b6de9edca2f133f3e7ad0c3e631eadd0d18e89bfdbfe0",
    "0x976ff83372d5e5f4618927de2f64ebc14cd0a2c651ddded4ba0485973aa03caa",
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
                    # => [CODE_ROOT, 0, 0, 0, 0]
    
                    exec.account::set_code
                    # => [0, 0, 0, 0]
                end
    
                # acct proc 6
                export.create_note
                    exec.tx::create_note
                    # => [ptr]
    
                    swap drop swap drop swap drop
                end
    
                # acct proc 7
                export.add_asset_to_note
                    exec.tx::add_asset_to_note
                    # => [ptr]
    
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
