use assembly::Assembler;

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 12] = [
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
    "0x8fe219b459e7cf3ced88a5666b3396f31cb97d4729d30b8c20aa9667099846c5",
    "0xe203ef16c929e5a9253b51d3e71dd182fb3b4b707a5ba5c89bd3e92e587c10b2",
    "0x92ed8069f0df10267fa4f8c6893c76072264f7b5bf9a6ae2c5dc772cb42f2ad3",
    "0x7d243d87a57897742a551d98fc166d2cc35ab955ea33c2065db278bf2e6fb91c",
    "0xe370ca786b01d2b64b97fa2b5300bc54adc1df2d6c8c4945067d3a8fce1f3be2",
    "0xb6303c8fefc51895d988a5a30272ce97bd2699453f81b2fb8042e0dd55b0bfca",
    "0x0dbb810e899800e1023ea8921463837484b10dcb4ca1657092245a93c9a08953",
    "0xc23a46da36a2290941ea32e5ae432a22f501699eb09f6ad0b91bf6c70eaa5e9b",
    "0x806c992fc7f366314a8723e94d74999d2a663b8291c7d15efd83ddf2b00e8c7d",
    "0x7cbd0632fba52b8e4003d5cc73376fb5807f2ad5072425563a74f5cf02a56250",
];

/*\
  0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5: account_procedure_1
  0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794: account_procedure_2
  0x8fe219b459e7cf3ced88a5666b3396f31cb97d4729d30b8c20aa9667099846c5: add_asset_to_note
  0xe203ef16c929e5a9253b51d3e71dd182fb3b4b707a5ba5c89bd3e92e587c10b2: create_note
  0x92ed8069f0df10267fa4f8c6893c76072264f7b5bf9a6ae2c5dc772cb42f2ad3: incr_nonce
  0x7d243d87a57897742a551d98fc166d2cc35ab955ea33c2065db278bf2e6fb91c: move_asset_to_note
  0xe370ca786b01d2b64b97fa2b5300bc54adc1df2d6c8c4945067d3a8fce1f3be2: receive_asset
  0xb6303c8fefc51895d988a5a30272ce97bd2699453f81b2fb8042e0dd55b0bfca: remove_asset
  0x0dbb810e899800e1023ea8921463837484b10dcb4ca1657092245a93c9a08953: send_asset
  0xc23a46da36a2290941ea32e5ae432a22f501699eb09f6ad0b91bf6c70eaa5e9b: set_code
  0x806c992fc7f366314a8723e94d74999d2a663b8291c7d15efd83ddf2b00e8c7d: set_item
  0x7cbd0632fba52b8e4003d5cc73376fb5807f2ad5072425563a74f5cf02a56250: set_map_item
*/

pub const ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT: &str = MASTS[2];
pub const ACCOUNT_SEND_ASSET_MAST_ROOT: &str = MASTS[8];
pub const ACCOUNT_INCR_NONCE_MAST_ROOT: &str = MASTS[4];
pub const ACCOUNT_SET_ITEM_MAST_ROOT: &str = MASTS[10];
pub const ACCOUNT_SET_MAP_ITEM_MAST_ROOT: &str = MASTS[11];
pub const ACCOUNT_SET_CODE_MAST_ROOT: &str = MASTS[9];
pub const ACCOUNT_REMOVE_ASSET_MAST_ROOT: &str = MASTS[7];

pub const ACCOUNT_ACCOUNT_PROCEDURE_1_MAST_ROOT: &str = MASTS[0];
pub const ACCOUNT_ACCOUNT_PROCEDURE_2_MAST_ROOT: &str = MASTS[1];

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
    export.::miden::contracts::wallets::basic::receive_asset
    export.::miden::contracts::wallets::basic::send_asset
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
";

pub const DEFAULT_AUTH_SCRIPT: &str = "
    begin
        call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
    end
";

impl AccountCode {
    /// Creates a mock [AccountCode] that exposes wallet interface
    pub fn mock_wallet(assembler: Assembler) -> AccountCode {
        let account_code = "\
        use.miden::account
        use.miden::tx

        # acct proc 0
        export.::miden::contracts::wallets::basic::receive_asset
        # acct proc 1
        export.::miden::contracts::wallets::basic::send_asset
        # acct proc 2
        export.::miden::contracts::wallets::basic::create_note
        # acct proc 3
        export.::miden::contracts::wallets::basic::move_asset_to_note

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
        export.add_asset_to_note
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]
        end

        # acct proc 9
        export.remove_asset
            exec.account::remove_asset
            # => [ASSET]
        end

        # acct proc 10
        export.account_procedure_1
            push.1.2
            add
        end

        # acct proc 11
        export.account_procedure_2
            push.2.1
            sub
        end
        ";

        let code = AccountCode::compile(account_code, assembler).unwrap();
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
        ];
        assert!(current == MASTS, "const MASTS: [&str; 12] = {:?};", current);

        code
    }

    /// Creates a mock [AccountCode] with default assembler and mock code
    pub fn mock() -> AccountCode {
        AccountCode::compile(CODE, Assembler::default()).unwrap()
    }
}
