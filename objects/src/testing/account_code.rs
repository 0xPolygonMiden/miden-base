use assembly::Assembler;

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 12] = [
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
    "0xd68cf600d75f4c8f47fde53e97dff4dd7fbf3abd0fee92fd92d09787075b7ad8",
    "0xdd55fc036505f12a47afa4cbe558f3e4eaee91e7fb4ad946042d2eb9c73a5ca4",
    "0xd25e1522abb81d7e1343dc0722514c587e0b0730de6f58888ca5d56e4332eae2",
    "0x8f6d91270bb15c57084af3f07db8209d6088f6841c6447c295fa3a300b81740e",
    "0x03bb20b622523663c1d47d823f50b97ae4d01a12bcdab3eff76473a570f08240",
    "0x53d0de6a04f92507d1ae4da76b1f215876cd6cd2062adce1ccc2ed74808d9157",
    "0x9f1275829532e8f5c200dfb369747261c635def6f20ba7478935cf67db3f4bff",
    "0xd544028d21f646d1793e51f834f9b50fa33c695cc3587a3511e88284f23bd0ad",
    "0xb859cc923ec62c650589d4653d1ec6c727daa3c87385c8c19fb843b95cae63dd",
    "0x43474615bbc6e34da5a1217ff98090df222d3491ed0bfe0aae127c0fc40de9d5",
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
