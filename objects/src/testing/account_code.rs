use assembly::Assembler;

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 12] = [
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
    "0xed87fcbdaf52f9d544d6362c0177f7294d917dfdbf48ee31b3df7b28be6f4ea0",
    "0xf26ab9001bbe615e3f39e6bf5c12dd32754b99d9476c6dd003881c3c52a6bfc1",
    "0x3f4b2dadf8a08e963898bc172f3fb1f4cbd7c560cb1d5f88eec582d0f9819edb",
    "0x31c806646881424c912cf5af6ad8002fe1f24cdc83e769525dd6c68df94a8f3b",
    "0x1b4fd4fc6d711276c4b25dc9e9b8dff6feb8f3762562fcecf45b85c7e4ed2437",
    "0x8197f4791b030c86bd4e3e4a241de840de47fa1ec338b8668ddb37052eee00a6",
    "0x16d574d4d5a70f83c86843a62a19ca647d2110c11cfd5cb2fc93a149f3c9c2c5",
    "0x70573a57c1b809e6feacf3bfeda3c6a22e4ef018979d2a0f9d466e54ed9c44a5",
    "0xa17f6ebb8988361106fa6395909da49fed668d9175a53b9fb71da101f749232c",
    "0x8e5e78723ffb6ddac06a9a3f6ccf9964c8559b84d773c46922db42d818bbe8f5",
];

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
        // Comparing all the values together, in case multiple of them change, a single test run
        // will detect it.
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

    /// Creates a mock [AccountCode] with specific code and assembler
    pub fn mock_with_code(source_code: &str, assembler: Assembler) -> AccountCode {
        Self::compile(source_code, assembler).unwrap()
    }

    /// Creates a mock [AccountCode] with default assembler and mock code
    pub fn mock() -> AccountCode {
        Self::compile(CODE, Assembler::default()).unwrap()
    }
}
