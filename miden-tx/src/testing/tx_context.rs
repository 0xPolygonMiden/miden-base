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
    assembly::{Assembler, ProgramAst},
    assets::{Asset, FungibleAsset},
    notes::{
        Note, NoteAssets, NoteId, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteType,
    },
    testing::{
        account_code::{ACCOUNT_ADD_ASSET_TO_NOTE_MAST_ROOT, ACCOUNT_CREATE_NOTE_MAST_ROOT},
        block::MockChain,
        constants::{
            CONSUMED_ASSET_1_AMOUNT, CONSUMED_ASSET_2_AMOUNT, CONSUMED_ASSET_3_AMOUNT,
            NON_FUNGIBLE_ASSET_DATA_2,
        },
        notes::{AssetPreservationStatus, NoteBuilder, DEFAULT_NOTE_CODE},
        prepare_word,
        storage::prepare_assets,
    },
    transaction::{
        InputNote, InputNotes, OutputNote, PreparedTransaction, TransactionArgs, TransactionInputs,
    },
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use vm_processor::{AdviceInputs, ExecutionError, Felt, Process, Word, ONE, ZERO};

use super::{executor::CodeExecutor, utils::create_test_chain, MockHost};

// TRANSACTION CONTEXT
// ================================================================================================

pub struct TransactionContext {
    mock_chain: MockChain,
    expected_output_notes: Vec<Note>,
    tx_args: TransactionArgs,
    tx_inputs: TransactionInputs,
    advice_inputs: AdviceInputs,
}

impl TransactionContext {
    pub fn execute_code(&self, code: &str) -> Result<Process<MockHost>, ExecutionError> {
        let tx = self.get_prepared_transaction(code);
        let (stack_inputs, mut advice_inputs) = tx.get_kernel_inputs();
        advice_inputs.extend(self.advice_inputs.clone());

        CodeExecutor::new(MockHost::new(tx.account().into(), advice_inputs))
            .stack_inputs(stack_inputs)
            .run(code)
    }

    pub fn execute_transaction(
        &self,
        tx: &PreparedTransaction,
    ) -> Result<Process<MockHost>, ExecutionError> {
        let (stack_inputs, advice_inputs) = tx.get_kernel_inputs();

        CodeExecutor::new(MockHost::new(tx.account().into(), advice_inputs))
            .stack_inputs(stack_inputs)
            .execute_program(tx.program().clone())
    }

    pub fn get_prepared_transaction(&self, code: &str) -> PreparedTransaction {
        let assembler = TransactionKernel::assembler().with_debug_mode(true);
        let program = assembler.compile(code).unwrap();
        PreparedTransaction::new(program, self.tx_inputs.clone(), self.tx_args.clone())
    }

    pub fn account(&self) -> &Account {
        self.tx_inputs.account()
    }

    pub fn account_seed(&self) -> Option<Word> {
        self.tx_inputs.account_seed()
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

    pub fn into_parts(self) -> (MockChain, Vec<Note>, TransactionArgs, TransactionInputs) {
        (self.mock_chain, self.expected_output_notes, self.tx_args, self.tx_inputs)
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

    pub fn assembler(mut self, assembler: Assembler) -> Self {
        self.assembler = assembler;
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

    pub fn add_input_note(mut self, input_note: Note) -> Self {
        self.input_notes.extend(vec![input_note]);
        self
    }

    pub fn tx_args(mut self, tx_args: TransactionArgs) -> Self {
        self.tx_args = tx_args;
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

    pub fn add_expected_output_note(mut self, output_note: OutputNote) -> Self {
        if let OutputNote::Full(note) = output_note {
            self.expected_output_notes.extend([note]);
        }
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
        let mock_chain = create_test_chain(self.input_notes.clone());
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
    let mut serial_num_gen = SerialNumGenerator::new();

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
        .note_inputs(vec![ONE])
        .unwrap()
        .add_asset(fungible_asset_1)
        .build(assembler)
        .unwrap();

    let created_note_2 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs(vec![Felt::new(2)])
        .unwrap()
        .add_asset(fungible_asset_2)
        .build(assembler)
        .unwrap();

    let created_note_3 = NoteBuilder::new(sender, ChaCha20Rng::from_rng(&mut rng).unwrap())
        .note_inputs(vec![Felt::new(3)])
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
    let note_1_script_ast = ProgramAst::parse(&note_1_script_src).unwrap();
    let (note_1_script, _) = NoteScript::new(note_1_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_1]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(1)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_1_script, inputs);
    let consumed_note_1 = Note::new(vault, metadata, recipient);

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
    let note_2_script_ast = ProgramAst::parse(&note_2_script_src).unwrap();
    let (note_2_script, _) = NoteScript::new(note_2_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_2, fungible_asset_3]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_2_script, inputs);
    let consumed_note_2 = Note::new(vault, metadata, recipient);

    let note_3_script_ast = ProgramAst::parse(DEFAULT_NOTE_CODE).unwrap();
    let (note_3_script, _) = NoteScript::new(note_3_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![fungible_asset_2, fungible_asset_3]).unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(2)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_3_script, inputs);
    let consumed_note_3 = Note::new(vault, metadata, recipient);

    let note_4_script_ast = ProgramAst::parse(DEFAULT_NOTE_CODE).unwrap();
    let (note_4_script, _) = NoteScript::new(note_4_script_ast, assembler).unwrap();
    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![Asset::mock_non_fungible(
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
        &NON_FUNGIBLE_ASSET_DATA_2,
    )])
    .unwrap();
    let inputs = NoteInputs::new(vec![Felt::new(1)]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_4_script, inputs);
    let consumed_note_4 = Note::new(vault, metadata, recipient);

    // note that changes the account vault
    let note_5_script_ast = ProgramAst::parse(
        "\
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
        ",
    )
    .unwrap();
    let (note_5_script, _) = NoteScript::new(note_5_script_ast, assembler).unwrap();

    let metadata = NoteMetadata::new(sender, NoteType::Public, 0.into(), ZERO).unwrap();
    let vault = NoteAssets::new(vec![
        fungible_asset_1,
        fungible_asset_3,
        Asset::mock_non_fungible(
            ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN,
            &NON_FUNGIBLE_ASSET_DATA_2,
        ),
    ])
    .unwrap();

    let inputs = NoteInputs::new(vec![]).unwrap();
    let recipient = NoteRecipient::new(serial_num_gen.next(), note_5_script, inputs);
    let consumed_note_5 = Note::new(vault, metadata, recipient);

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

struct SerialNumGenerator {
    state: u64,
}

impl SerialNumGenerator {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn next(&mut self) -> Word {
        let serial_num = [
            Felt::new(self.state),
            Felt::new(self.state + 1),
            Felt::new(self.state + 2),
            Felt::new(self.state + 3),
        ];
        self.state += 4;
        serial_num
    }
}
