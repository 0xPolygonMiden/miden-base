use alloc::vec::Vec;

use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2,
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3, ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_SENDER,
        },
        Account, AccountCode, AccountId,
    },
    assembly::{Assembler, ModuleAst},
    assets::{Asset, FungibleAsset},
    notes::{Note, NoteId, NoteType},
    testing::{
        account_code::{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT, ACCOUNT_CREATE_NOTE_MAST_ROOT},
        block::{MockChain, MockChainBuilder},
        constants::{
            CONSUMED_ASSET_1_AMOUNT, CONSUMED_ASSET_2_AMOUNT, CONSUMED_ASSET_3_AMOUNT,
            NON_FUNGIBLE_ASSET_DATA_2,
        },
        notes::{AssetPreservationStatus, NoteBuilder},
        prepare_word,
        storage::prepare_assets,
    },
    transaction::{
        InputNote, InputNotes, OutputNote, PreparedTransaction, TransactionArgs, TransactionInputs,
    },
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use vm_processor::{AdviceInputs, ExecutionError, Felt, Process, Word};
use winter_maybe_async::maybe_async;

use super::{executor::CodeExecutor, MockHost};
use crate::{DataStore, DataStoreError};

// TRANSACTION CONTEXT
// ================================================================================================

#[derive(Debug, Clone)]
pub struct TransactionContext {
    mock_chain: MockChain,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    advice_inputs: AdviceInputs,
}

impl TransactionContext {
    pub fn execute_code(&self, code: &str) -> Result<Process<MockHost>, ExecutionError> {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let program = assembler.compile(code).unwrap();
        let tx = PreparedTransaction::new(program, self.tx_inputs.clone(), self.tx_args.clone());
        let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
        advice_inputs.extend(self.advice_inputs.clone());

        CodeExecutor::new(MockHost::new(tx.account().into(), advice_inputs))
            .stack_inputs(stack_inputs)
            .run(code)
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    pub fn expected_output_notes(&self) -> &[Note] {
        &self.expected_output_notes
    }

    pub fn mock_chain(&self) -> &MockChain {
        &self.mock_chain
    }

    pub fn input_notes(&self) -> InputNotes<InputNote> {
        InputNotes::new(self.mock_chain.available_notes().clone()).unwrap()
    }

    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }

    pub fn set_tx_args(&mut self, tx_args: TransactionArgs) {
        self.tx_args = tx_args;
    }

    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }
}

impl DataStore for TransactionContext {
    #[maybe_async]
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        assert_eq!(block_num, self.tx_inputs.block_header().block_num());
        assert_eq!(notes.len(), self.tx_inputs.input_notes().num_notes());

        Ok(self.tx_inputs.clone())
    }

    #[maybe_async]
    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.tx_inputs.account().id());
        Ok(self.tx_inputs.account().code().module().clone())
    }
}

// TRANSACTION CONTEXT BUILDER
// ================================================================================================

pub struct TransactionContextBuilder {
    assembler: Assembler,
    account: Account,
    account_seed: Option<Word>,
    advice_inputs: Option<AdviceInputs>,
    input_notes: Vec<Note>,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
}

impl TransactionContextBuilder {
    pub fn new(account: Account) -> Self {
        let tx_args = TransactionArgs::default();
        Self {
            assembler: TransactionKernel::assembler().with_debug_mode(true),
            account,
            account_seed: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args,
            advice_inputs: None,
        }
    }

    pub fn with_standard_account(account_id: u64, nonce: Felt) -> Self {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let account = Account::mock(account_id, nonce, AccountCode::mock_wallet(&assembler));

        Self {
            assembler,
            account,
            account_seed: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args: TransactionArgs::default(),
            advice_inputs: None,
        }
    }

    pub fn with_fungible_faucet(acct_id: u64, nonce: Felt, initial_balance: Felt) -> Self {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let account = Account::mock_fungible_faucet(acct_id, nonce, initial_balance, &assembler);

        Self {
            assembler,
            account,
            account_seed: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args: TransactionArgs::default(),
            advice_inputs: None,
        }
    }

    pub fn with_non_fungible_faucet(acct_id: u64, nonce: Felt, empty_reserved_slot: bool) -> Self {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let account =
            Account::mock_non_fungible_faucet(acct_id, nonce, empty_reserved_slot, &assembler);

        Self {
            assembler,
            account,
            account_seed: None,
            input_notes: Vec::new(),
            expected_output_notes: Vec::new(),
            tx_args: TransactionArgs::default(),
            advice_inputs: None,
        }
    }

    pub fn account_seed(mut self, account_seed: Word) -> Self {
        self.account_seed = Some(account_seed);
        self
    }

    pub fn advice_inputs(mut self, advice_inputs: AdviceInputs) -> Self {
        self.advice_inputs = Some(advice_inputs);
        self
    }

    pub fn input_notes(mut self, input_notes: Vec<Note>) -> Self {
        self.input_notes.extend(input_notes);
        self
    }

    pub fn expected_notes(mut self, output_notes: Vec<OutputNote>) -> Self {
        let output_notes = output_notes.into_iter().filter_map(|n| match n {
            OutputNote::Full(note) => Some(note),
            OutputNote::Partial(_) => None,
            OutputNote::Header(_) => None,
        });

        self.expected_output_notes.extend(output_notes);
        self
    }

    /// Populates input and expected notes.
    pub fn with_mock_notes(self, asset_preservation: AssetPreservationStatus) -> Self {
        let (mut input_notes, output_notes) = mock_notes(&self.assembler);

        let consumed_note_5 = input_notes.pop().unwrap();
        let consumed_note_4 = input_notes.pop().unwrap();
        let consumed_note_3 = input_notes.pop().unwrap();
        let consumed_note_2 = input_notes.pop().unwrap();
        let consumed_note_1 = input_notes.pop().unwrap();

        let notes = match asset_preservation {
            AssetPreservationStatus::TooFewInput => vec![consumed_note_1],
            AssetPreservationStatus::Preserved => {
                vec![consumed_note_1, consumed_note_2]
            },
            AssetPreservationStatus::PreservedWithAccountVaultDelta => {
                vec![consumed_note_1, consumed_note_2, consumed_note_5]
            },
            AssetPreservationStatus::TooManyFungibleInput => {
                vec![consumed_note_1, consumed_note_2, consumed_note_3]
            },
            AssetPreservationStatus::TooManyNonFungibleInput => {
                vec![consumed_note_1, consumed_note_2, consumed_note_4]
            },
        };

        self.input_notes(notes).expected_notes(output_notes)
    }

    pub fn build(mut self) -> TransactionContext {
        let mut mock_chain = MockChainBuilder::new().notes(self.input_notes.clone()).build();
        mock_chain.seal_block();
        mock_chain.seal_block();
        mock_chain.seal_block();

        let input_note_ids: Vec<NoteId> =
            mock_chain.available_notes().iter().map(|n| n.id()).collect();

        let tx_inputs = mock_chain.get_transaction_inputs(
            self.account.clone(),
            self.account_seed,
            &input_note_ids,
        );

        self.tx_args.extend_expected_output_notes(self.expected_output_notes.clone());

        TransactionContext {
            mock_chain,
            expected_output_notes: self.expected_output_notes,
            tx_args: self.tx_args,
            tx_inputs,
            advice_inputs: self.advice_inputs.unwrap_or_default(),
        }
    }
}

fn mock_notes(assembler: &Assembler) -> (Vec<Note>, Vec<OutputNote>) {
    // ACCOUNT IDS
    // --------------------------------------------------------------------------------------------
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_1).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_2).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN_3).unwrap();

    // ASSETS
    // --------------------------------------------------------------------------------------------
    let fungible_asset_1: Asset =
        FungibleAsset::new(faucet_id_1, CONSUMED_ASSET_1_AMOUNT).unwrap().into();
    let fungible_asset_2: Asset =
        FungibleAsset::new(faucet_id_2, CONSUMED_ASSET_2_AMOUNT).unwrap().into();
    let fungible_asset_3: Asset =
        FungibleAsset::new(faucet_id_3, CONSUMED_ASSET_3_AMOUNT).unwrap().into();

    // CREATED NOTES
    // --------------------------------------------------------------------------------------------
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);
    let created_note_1 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([1u32.into()])
        .unwrap()
        .add_asset(fungible_asset_1)
        .build(assembler)
        .unwrap();

    let created_note_2 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([2u32.into()])
        .unwrap()
        .add_asset(fungible_asset_2)
        .build(assembler)
        .unwrap();

    let created_note_3 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([3u32.into()])
        .unwrap()
        .add_asset(fungible_asset_3)
        .build(assembler)
        .unwrap();

    // CONSUMED NOTES
    // --------------------------------------------------------------------------------------------

    let note_1_script_src = format!(
        "\
        begin
            # create note 0
            push.{recipient0}
            push.{PUBLIC_NOTE}
            push.{aux0}
            push.{tag0}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}

            push.{asset0} movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            dropw dropw dropw

            # create note 1
            push.{recipient1}
            push.{PUBLIC_NOTE}
            push.{aux1}
            push.{tag1}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}

            push.{asset1} movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            dropw dropw dropw
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        recipient0 = prepare_word(&created_note_1.recipient().digest()),
        aux0 = created_note_1.metadata().aux(),
        tag0 = created_note_1.metadata().tag(),
        asset0 = prepare_assets(created_note_1.assets())[0],
        recipient1 = prepare_word(&created_note_2.recipient().digest()),
        aux1 = created_note_2.metadata().aux(),
        tag1 = created_note_2.metadata().tag(),
        asset1 = prepare_assets(created_note_2.assets())[0],
    );

    let consumed_note_1 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([1u32.into()])
        .unwrap()
        .add_asset(fungible_asset_1)
        .code(note_1_script_src)
        .build(assembler)
        .unwrap();

    let note_2_script_src = format!(
        "\
        begin
            # create note 2
            push.{recipient}
            push.{PUBLIC_NOTE}
            push.{aux}
            push.{tag}
            # MAST root of the `create_note` mock account procedure
            call.{ACCOUNT_CREATE_NOTE_MAST_ROOT}

            push.{asset} movup.4
            call.{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT}
            dropw dropw dropw
        end
        ",
        PUBLIC_NOTE = NoteType::Public as u8,
        recipient = prepare_word(&created_note_3.recipient().digest()),
        aux = created_note_3.metadata().aux(),
        tag = created_note_3.metadata().tag(),
        asset = prepare_assets(created_note_3.assets())[0],
    );

    let consumed_note_2 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([2u32.into()])
        .unwrap()
        .add_asset(fungible_asset_2)
        .add_asset(fungible_asset_3)
        .code(note_2_script_src)
        .build(assembler)
        .unwrap();

    let consumed_note_3 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([2u32.into()])
        .unwrap()
        .add_asset(fungible_asset_2)
        .add_asset(fungible_asset_3)
        .build(assembler)
        .unwrap();

    let consumed_note_4 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs([1u32.into()])
        .unwrap()
        .add_asset(Asset::mock_non_fungible(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA_2,
        ))
        .build(assembler)
        .unwrap();

    // note that changes the account vault
    let note_5_script_src = "\
        use.miden::note
        use.miden::contracts::wallets::basic->wallet

        begin
            # read the assets to memory
            push.0 exec.note::get_assets
            # => [num_assets, dest_ptr]

            # assert the number of assets is 3
            push.3 assert_eq
            # => [dest_ptr]

            # add the first asset to the vault
            padw dup.4 mem_loadw call.wallet::receive_asset dropw
            # => [dest_ptr]

            # add the second asset to the vault
            push.1 add padw dup.4 mem_loadw call.wallet::receive_asset dropw
            # => [dest_ptr+1]

            # add the third asset to the vault
            push.1 add padw movup.4 mem_loadw call.wallet::receive_asset dropw
            # => []
        end
        ";

    let consumed_note_5 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .add_asset(fungible_asset_1)
        .add_asset(fungible_asset_3)
        .add_asset(Asset::mock_non_fungible(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA_2,
        ))
        .code(note_5_script_src)
        .build(assembler)
        .unwrap();

    let consumed_notes = vec![
        consumed_note_1,
        consumed_note_2,
        consumed_note_3,
        consumed_note_4,
        consumed_note_5,
    ];
    let output_notes = vec![
        OutputNote::Full(created_note_1),
        OutputNote::Full(created_note_2),
        OutputNote::Full(created_note_3),
    ];

    (consumed_notes, output_notes)
}
