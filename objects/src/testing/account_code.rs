use assembly::Assembler;

use crate::accounts::AccountCode;

// The MAST root of the default account's interface. Use these constants to interact with the
// account's procedures.
const MASTS: [&str; 12] = [
    "0xff06b90f849c4b262cbfbea67042c4ea017ea0e9c558848a951d44b23370bec5",
    "0x8ef0092134469a1330e3c468f57c7f085ce611645d09cc7516c786fefc71d794",
    "0xa69d960467d9484c79812394587b81b13258a8c76ce7e458229323c734136bf0",
    "0x68d503aba05d48bfb8ed4647a54df4e2f4f3f380868300710f991e201407ae36",
    "0x18af4d279042d8042eadb854bb58e546bc7546a0b37f34dfd2fc7f995f8f33ce",
    "0xaeeed1fe7453d66dd0f175fa1e4a9534eaa9928ee4ba6cc41ea27f41ec619038",
    "0x5a600fe00f66dd29083a2985e310da0f85cfc90e5faf4c7661a5c2a91ebcde69",
    "0x5c739834a3f5d34fc4c27bc78fd72939a9fa98cce7c5d6bb477624e3431b22ee",
    "0xfb2afc65a25b322cdf51bf5450e4c32b93fdde19da351ac8cf28145f19969016",
    "0xe1d6c52a75a15cee295e9f787a8bd0c73996b878b746f9c9f7ea9a7af83e6e50",
    "0x1798f2959f525fc9b55b0307542600fb987afd965533c993cffa73461e1a1be2",
    "0x0651c2b7081d29268e28fe2b99da0cebccd9515d228974a1b3959b12b3a4a448",
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
