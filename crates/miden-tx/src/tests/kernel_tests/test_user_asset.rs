use alloc::string::String;

use miden_lib::transaction::{
    TransactionKernel,
    memory::{
        ASSET_BOOKKEEPING_SIZE, ASSET_ISSUER_PREFIX_OFFSET, ASSET_MIN_PTR, ASSET_NEXT_OFFSET_PTR,
        ASSET_PTR_MAP_MIN,
    },
};
use miden_objects::{
    account::{AccountBuilder, AccountComponent},
    assembly::{Compile, LibraryPath},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use vm_processor::{ContextId, Felt, ProcessState};

use crate::{
    testing::MockChain,
    tests::kernel_tests::{read_root_mem_word, test_fpi::get_mock_fpi_adv_inputs},
};

#[test]
fn test_fpi_asset_memory() {
    // Num fields must satisfy num_fields % 8 == 0.

    // Randomly picked a non-zero value for demo purposes.
    const TREASURY_CAP_ASSET_TYPE: u32 = 3;
    const TREASURY_CAP_NUM_FIELDS: u32 = 8;
    const TREASURY_CAP_FIELD_TYPE_ID_PREFIX: u32 = 0;
    const TREASURY_CAP_FIELD_TYPE_ID_SUFFIX: u32 = 1;
    const TREASURY_CAP_FIELD_OTW_ID: u32 = 2;
    const TREASURY_CAP_FIELD_TOTAL_SUPPLY: u32 = 3;

    const TOKEN_ASSET_TYPE: u32 = 5;
    const TOKEN_NUM_FIELDS: u32 = 8;
    const TOKEN_FIELD_TYPE_ID_PREFIX: u32 = 0;
    const TOKEN_FIELD_TYPE_ID_SUFFIX: u32 = 1;
    const TOKEN_FIELD_OTW_ID: u32 = 2;
    const TOKEN_FIELD_AMOUNT: u32 = 3;

    let miden_std_account_code = format!(
        "
        use.miden::account
        use.miden::asset

        const.TREASURY_CAP_ASSET_TYPE={TREASURY_CAP_ASSET_TYPE}
        const.TREASURY_CAP_NUM_FIELDS={TREASURY_CAP_NUM_FIELDS}
        const.TREASURY_CAP_FIELD_TYPE_ID_PREFIX={TREASURY_CAP_FIELD_TYPE_ID_PREFIX}
        const.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX={TREASURY_CAP_FIELD_TYPE_ID_SUFFIX}
        const.TREASURY_CAP_FIELD_OTW_ID={TREASURY_CAP_FIELD_OTW_ID}
        const.TREASURY_CAP_FIELD_TOTAL_SUPPLY={TREASURY_CAP_FIELD_TOTAL_SUPPLY}

        const.TOKEN_ASSET_TYPE={TOKEN_ASSET_TYPE}
        const.TOKEN_NUM_FIELDS={TOKEN_NUM_FIELDS}
        const.TOKEN_FIELD_TYPE_ID_PREFIX={TOKEN_FIELD_TYPE_ID_PREFIX}
        const.TOKEN_FIELD_TYPE_ID_SUFFIX={TOKEN_FIELD_TYPE_ID_SUFFIX}
        const.TOKEN_FIELD_OTW_ID={TOKEN_FIELD_OTW_ID}
        const.TOKEN_FIELD_AMOUNT={TOKEN_FIELD_AMOUNT}

        # TREASURY CAP
        # =========================================================================================

        #! Inputs:  []
        #! Outputs: [treasury_cap_ptr]
        export.create
            push.TREASURY_CAP_ASSET_TYPE.TREASURY_CAP_NUM_FIELDS
            exec.asset::create
            # => [treasury_cap_ptr, otw_id]

            # consume OTW or abort if it was already consumed
            dup.1 exec.asset::consume_one_time_witness
            # => [treasury_cap_ptr, otw_id]

            swap push.TREASURY_CAP_FIELD_OTW_ID dup.2
            # => [treasury_cap_ptr, field_idx, otw_id, treasury_cap_ptr]
            exec.asset::set_field
            # => [treasury_cap_ptr]

            exec.account::get_native_id
            # => [native_id_prefix, native_id_suffix, treasury_cap_ptr]
            dup.2
            # => [treasury_cap_ptr, native_id_prefix, native_id_suffix, treasury_cap_ptr]

            push.TREASURY_CAP_FIELD_TYPE_ID_PREFIX swap
            # => [treasury_cap_ptr, field_idx, native_id_prefix, native_id_suffix, treasury_cap_ptr]
            exec.asset::set_field
            # => [native_id_suffix, treasury_cap_ptr]

            push.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX dup.2
            # => [treasury_cap_ptr, field_idx, native_id_suffix, treasury_cap_ptr]
            exec.asset::set_field
            # => [treasury_cap_ptr]

            # truncate the stack
            swap drop
        end

        #! Inputs:  [treasury_cap_ptr, amount]
        #! Outputs: [token_ptr]
        export.mint
            dup exec.assert_treasury_cap
            # => [treasury_cap_ptr, amount]

            push.TOKEN_ASSET_TYPE.TOKEN_NUM_FIELDS
            exec.asset::create
            # => [token_ptr, treasury_cap_ptr, amount]

            # copy token flavour from treasury cap to token
            push.TREASURY_CAP_FIELD_OTW_ID dup.2 exec.asset::get_field
            # => [otw_id, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_OTW_ID dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            push.TREASURY_CAP_FIELD_TYPE_ID_PREFIX dup.2 exec.asset::get_field
            # => [issuer_prefix, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_TYPE_ID_PREFIX dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            push.TREASURY_CAP_FIELD_TYPE_ID_SUFFIX dup.2 exec.asset::get_field
            # => [issuer_prefix, token_ptr, treasury_cap_ptr, amount]
            push.TOKEN_FIELD_TYPE_ID_SUFFIX dup.2
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            # set amount on token
            dup.2 push.TOKEN_FIELD_AMOUNT dup.2
            # => [token_ptr, amount_field_idx, amount, token_ptr, treasury_cap_ptr, amount]
            exec.asset::set_field
            # => [token_ptr, treasury_cap_ptr, amount]

            # increase total supply in treasury
            push.TREASURY_CAP_FIELD_TOTAL_SUPPLY dup.2 exec.asset::get_field
            # => [total_supply, token_ptr, treasury_cap_ptr, amount]
            movup.3 add
            # => [new_total_supply, token_ptr, treasury_cap_ptr]
            push.TREASURY_CAP_FIELD_TOTAL_SUPPLY movup.3 exec.asset::set_field
            # => [token_ptr]
        end

        # TOKEN
        # =========================================================================================

        #! Inputs:  [token_ptr]
        #! Outputs: [TOKEN_ASSET_ID]
        export.store_to_account
            dup exec.assert_token
            # => [token_ptr]

            # before allowing the store operation we could check that the calling account's ID
            # is in the storage of this account to implement a regulated token
            # we could also prevent moving entirely by not exposing a procedure that wraps store_to_account
            # (or an equivalent store_to_note)

            dup exec.asset::store_to_account
            # => [asset_ptr]

            exec.asset::get_id
            # => [ASSET_ID]
        end

        #! Inputs:  [TOKEN_ASSET_ID]
        #! Outputs: [token_ptr]
        export.load_from_account
          # pass the type of the asset so the tx kernel can validate the type.
          push.TOKEN_ASSET_TYPE movdn.4
          # => [ASSET_ID, asset_type]

          exec.asset::load_from_account
          # => [token_ptr]
        end

        # HELPERS
        # =========================================================================================

        #! Inputs:  [treasury_cap_ptr]
        #! Outputs: []
        proc.assert_treasury_cap
          dup exec.assert_asset_issuer
          # => [treasury_cap_ptr]

          exec.asset::get_asset_type push.TREASURY_CAP_ASSET_TYPE assert_eq.err=13844
          # => []
        end

        #! Inputs:  [token_ptr]
        #! Outputs: []
        proc.assert_token
          dup exec.assert_asset_issuer
          # => [treasury_cap_ptr]

          exec.asset::get_asset_type push.TOKEN_ASSET_TYPE assert_eq.err=13845
          # => []
        end

        #! Inputs:  [asset_ptr]
        #! Outputs: []
        proc.assert_asset_issuer
          exec.asset::get_asset_issuer
          # => [asset_account_id_prefix, asset_account_id_suffix]

          exec.account::get_id
          # => [current_account_id_prefix, current_account_id_suffix, asset_account_id_prefix, asset_account_id_suffix]

          exec.account::is_id_equal assert.err=3421
          # => []
        end
    "
    );

    let miden_std =
        NamedModule::new(LibraryPath::new("miden_std::token").unwrap(), miden_std_account_code);
    let miden_std = TransactionKernel::testing_assembler()
        .with_debug_mode(true)
        .assemble_library([miden_std])
        .unwrap();

    let miden_std_account_component = AccountComponent::new(miden_std.clone(), vec![])
        .unwrap()
        .with_supports_all_types();

    let miden_std_account = AccountBuilder::new(ChaCha20Rng::from_os_rng().random())
        .with_component(miden_std_account_component)
        .build_existing()
        .unwrap();

    let mut mock_chain = MockChain::with_accounts(&[miden_std_account.clone()]);
    let native_account = mock_chain.add_new_wallet(crate::testing::Auth::BasicAuth);
    mock_chain.seal_next_block();
    let advice_inputs = get_mock_fpi_adv_inputs(vec![&miden_std_account], &mock_chain);

    const BOB_TOKEN_OTW: u32 = 8;

    let tx_code = format!(
        "
        use.miden::tx
        use.miden::asset
        use.miden_std::token
        use.kernel::prologue

        #! Inputs:  []
        #! Outputs: [treasury_cap_ptr]
        proc.create_bob_treasury_cap
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # Push OTW
            push.{BOB_TOKEN_OTW}
            # => [otw_id, pad(14)]

            # get the hash of the `create` procedure of the miden_std account
            procref.token::create

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, pad(15)]

            exec.tx::execute_foreign_procedure
            # => [treasury_cap_ptr]

            # truncate the stack
            movdn.15 dropw dropw dropw drop drop drop
            # => [treasury_cap_ptr]
        end

        #! Inputs:  [treasury_cap_ptr]
        #! Outputs: [token_ptr]
        proc.mint_bob_token
            # pad the stack for the `execute_foreign_procedure` execution
            push.0 padw padw padw movup.13
            # => [treasury_cap_ptr, pad(13)]

            # Push the amount of tokens to mint
            push.100 swap
            # => [treasury_cap_ptr, 100, pad(13)]

            # get the hash of the `mint` procedure of the miden_std account
            procref.token::mint

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, treasury_cap_ptr, 100, pad(13)]

            exec.tx::execute_foreign_procedure
            # => [token_ptr]

            # truncate the stack
            movdn.13 dropw dropw dropw drop
            # => [token_ptr]
        end

        #! Inputs:  [asset_ptr]
        #! Outputs: [ASSET_ID]
        proc.store_bob_token_to_account
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0.0 movup.15
            # => [asset_ptr, pad(15)]

            # get the hash of the `store_to_account` procedure of the miden_std account
            procref.token::store_to_account

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, asset_ptr, pad(15)]

            exec.tx::execute_foreign_procedure
            # => [ASSET_ID]

            # truncate the stack
            swapdw dropw dropw swapw dropw
            # => [ASSET_ID]
        end

        #! Inputs:  [ASSET_ID]
        #! Outputs: [asset_ptr]
        proc.load_bob_token_from_account
            # pad the stack for the `execute_foreign_procedure` execution
            padw swapw padw padw swapdw
            # => [ASSET_ID, pad(12)]

            # get the hash of the `load_from_account` procedure of the miden_std account
            procref.token::load_from_account

            # push the miden_std account ID
            push.{miden_std_suffix}.{miden_std_prefix}
            # => [miden_std_id_prefix, miden_std_id_suffix, FOREIGN_PROC_ROOT, ASSET_ID, pad(15)]

            exec.tx::execute_foreign_procedure
            # => [asset_ptr]

            # truncate the stack
            movdn.15 dropw dropw dropw drop drop drop
            # => [asset_ptr]
        end

        begin
            exec.prologue::prepare_transaction

            exec.create_bob_treasury_cap
            # => [treasury_cap_ptr]

            dup exec.mint_bob_token
            # => [token_ptr, treasury_cap_ptr]

            exec.store_bob_token_to_account
            # => [ASSET_ID, treasury_cap_ptr]

            exec.load_bob_token_from_account
            # => [token_ptr, treasury_cap_ptr]

            # truncate stack
            swapw dropw
        end
        ",
        miden_std_prefix = miden_std_account.id().prefix().as_felt(),
        miden_std_suffix = miden_std_account.id().suffix(),
    );

    let mut tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .foreign_account_codes(vec![miden_std_account.code().clone()])
        .advice_inputs(advice_inputs.clone())
        .build();

    tx_context.assembler_mut().set_debug_mode(true);
    tx_context.assembler_mut().add_library(miden_std).unwrap();
    let process = &tx_context.execute_code(&tx_code).unwrap();

    let token_ptr = u32::try_from(process.stack.get(0)).unwrap();
    let treasury_cap_ptr = u32::try_from(process.stack.get(1)).unwrap();

    // Dereference the pointers once to get the actual pointers.
    let token_ptr = read_mem_felt(process, token_ptr).as_int() as u32;
    let treasury_cap_ptr = read_mem_felt(process, treasury_cap_ptr).as_int() as u32;

    let next_offset = read_mem_felt(process, ASSET_NEXT_OFFSET_PTR).as_int() as u32;
    // We've created 2 assets and loaded 1.
    assert_eq!(next_offset, 3);
    assert_eq!(
        read_mem_felt(process, ASSET_PTR_MAP_MIN + next_offset).as_int() as u32,
        ASSET_MIN_PTR + 3 * ASSET_BOOKKEEPING_SIZE + 2 * TOKEN_NUM_FIELDS + TREASURY_CAP_NUM_FIELDS
    );

    // TREASURY CAP MEMORY ASSERTIONS
    assert_eq!(
        read_root_mem_word(&process.into(), treasury_cap_ptr + ASSET_ISSUER_PREFIX_OFFSET),
        [
            miden_std_account.id().prefix().as_felt(),
            miden_std_account.id().suffix(),
            Felt::from(TREASURY_CAP_ASSET_TYPE),
            Felt::from(TREASURY_CAP_NUM_FIELDS),
        ]
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TYPE_ID_PREFIX
        ),
        native_account.id().prefix().as_felt()
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TYPE_ID_SUFFIX
        ),
        native_account.id().suffix()
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_OTW_ID
        ),
        Felt::from(BOB_TOKEN_OTW)
    );
    assert_eq!(
        read_mem_felt(
            process,
            treasury_cap_ptr + ASSET_BOOKKEEPING_SIZE + TREASURY_CAP_FIELD_TOTAL_SUPPLY
        ),
        Felt::from(100u32)
    );

    // TOKEN MEMORY ASSERTIONS
    assert_eq!(
        read_root_mem_word(&process.into(), token_ptr + ASSET_ISSUER_PREFIX_OFFSET),
        [
            miden_std_account.id().prefix().as_felt(),
            miden_std_account.id().suffix(),
            Felt::from(TOKEN_ASSET_TYPE),
            Felt::from(TOKEN_NUM_FIELDS),
        ]
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_TYPE_ID_PREFIX),
        native_account.id().prefix().as_felt()
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_TYPE_ID_SUFFIX),
        native_account.id().suffix()
    );
    assert_eq!(
        read_mem_felt(process, token_ptr + ASSET_BOOKKEEPING_SIZE + TOKEN_FIELD_OTW_ID),
        Felt::from(BOB_TOKEN_OTW)
    );
}

pub fn read_mem_felt<'process>(process: impl Into<ProcessState<'process>>, addr: u32) -> Felt {
    process.into().get_mem_value(ContextId::root(), addr).unwrap()
}

struct NamedModule {
    lib_path: LibraryPath,
    code: String,
}

impl NamedModule {
    pub fn new(lib_path: LibraryPath, code: impl Into<String>) -> Self {
        Self { lib_path, code: code.into() }
    }
}

impl Compile for NamedModule {
    fn compile_with_options(
        self,
        source_manager: &dyn assembly::SourceManager,
        mut options: assembly::CompileOptions,
    ) -> Result<std::prelude::v1::Box<miden_objects::assembly::Module>, assembly::Report> {
        options.path = Some(self.lib_path);
        self.code.compile_with_options(source_manager, options)
    }
}
