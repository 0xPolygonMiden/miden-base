use std::sync::Arc;

use assembly::{ast::Module, Assembler, Library, LibraryPath};

use crate::accounts::AccountCode;

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
    /// Creates a mock [AccountCode]
    pub fn mock_account_code(assembler: Assembler, is_faucet: bool) -> AccountCode {
        AccountCode::new(Self::mock_library(assembler), is_faucet).unwrap()
    }

    /// Creates a mock [Library] which can be used to assemble programs and as a library to create a
    /// mock [AccountCode] interface. Transaction and note scripts that make use of this interface
    /// should be assembled with this.
    pub fn mock_library(assembler: Assembler) -> Library {
        let code = "
        use.miden::account
        use.miden::faucet
        use.miden::tx
        export.::miden::contracts::wallets::basic::receive_asset
        export.::miden::contracts::wallets::basic::send_asset
        export.::miden::contracts::wallets::basic::create_note
        export.::miden::contracts::wallets::basic::move_asset_to_note

        export.incr_nonce
            push.0 swap
            # => [value, 0]
            exec.account::incr_nonce
            # => [0]
        end

        export.set_item
            exec.account::set_item
            # => [R', V, 0, 0, 0]
            movup.8 drop movup.8 drop movup.8 drop
            # => [R', V]
        end

        export.get_item
            exec.account::get_item
            movup.8 drop movup.8 drop movup.8 drop
        end

        export.set_map_item
            exec.account::set_map_item
            # => [R', V, 0, 0, 0]
            movup.8 drop movup.8 drop movup.8 drop
            # => [R', V]
        end

        export.get_map_item
            exec.account::get_map_item
        end

        export.set_code
            padw swapw
            # => [CODE_COMMITMENT, 0, 0, 0, 0]
            exec.account::set_code
            # => [0, 0, 0, 0]
        end

        export.add_asset_to_note
            exec.tx::add_asset_to_note
            # => [ASSET, note_idx]
        end

        export.add_asset
            exec.account::add_asset
        end

        export.remove_asset
            exec.account::remove_asset
            # => [ASSET]
        end

        export.account_procedure_1
            push.1.2
            add
        end

        export.account_procedure_2
            push.2.1
            sub
        end

        export.mint
            exec.faucet::mint
        end

        export.burn
            exec.faucet::burn
        end
        ";
        let source_manager = Arc::new(assembly::DefaultSourceManager::default());
        let module = Module::parser(assembly::ast::ModuleKind::Library)
            .parse_str(LibraryPath::new("test::account").unwrap(), code, &source_manager)
            .unwrap();

        assembler.assemble_library(&[*module]).unwrap()
    }

    /// Creates a mock [AccountCode] with specific code and assembler
    pub fn mock_with_code(source_code: &str, assembler: Assembler) -> AccountCode {
        Self::compile(source_code, assembler, false).unwrap()
    }

    /// Creates a mock [AccountCode] with default assembler and mock code
    pub fn mock() -> AccountCode {
        Self::compile(CODE, Assembler::default(), false).unwrap()
    }
}
