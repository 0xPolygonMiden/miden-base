extern crate alloc;
pub use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_lib::transaction::TransactionTrace;
use miden_objects::{
    accounts::{Account, AccountId, AccountStub},
    assembly::ModuleAst,
    notes::NoteId,
    transaction::{ChainMmr, InputNote, InputNotes, TransactionArgs},
    BlockHeader,
};
use miden_tx::{DataStore, DataStoreError, TransactionHost, TransactionInputs};
use mock::mock::{
    account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, ExecutionError, Host, HostResponse,
    ProcessState,
};

// CONSTANTS
// ================================================================================================

/// Number of cycles needed to create an empty span whithout changing the stack state.
const SPAN_CREATION_SHIFT: u32 = 2;

// BENCHMARK HOST
// ================================================================================================

/// Wrapper around [TransactionHost] used for benchmarking: `tx_progress` allows to track the
/// progression of the transaction execution.
pub struct BenchHost<A: AdviceProvider> {
    host: TransactionHost<A>,
    tx_progress: TransactionProgress,
}

impl<A: AdviceProvider> BenchHost<A> {
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        Self {
            host: TransactionHost::new(account, adv_provider),
            tx_progress: TransactionProgress::default(),
        }
    }
}

impl<A: AdviceProvider> Host for BenchHost<A> {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.host.get_advice(process, extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        self.host.set_advice(process, injector)
    }

    fn on_trace<S: ProcessState>(
        &mut self,
        process: &S,
        trace_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        let event = TransactionTrace::try_from(trace_id)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        use TransactionTrace::*;
        match event {
            PrologueStart => self.tx_progress.prologue.set_start(process.clk()),
            PrologueEnd => self.tx_progress.prologue.set_end(process.clk()),
            NotesProcessingStart => self.tx_progress.notes_processing.set_start(process.clk()),
            NotesProcessingEnd => self.tx_progress.notes_processing.set_end(process.clk()),
            NoteConsumingStart => {
                self.tx_progress.note_consuming.push(CycleInterval::new(process.clk()))
            },
            NoteConsumingEnd => {
                if let Some(interval) = self.tx_progress.note_consuming.last_mut() {
                    interval.set_end(process.clk())
                }
            },
            TxScriptProcessingStart => {
                self.tx_progress.tx_script_processing.set_start(process.clk())
            },
            TxScriptProcessingEnd => self.tx_progress.tx_script_processing.set_end(process.clk()),
            EpilogueStart => self.tx_progress.epilogue.set_start(process.clk()),
            EpilogueEnd => self.tx_progress.epilogue.set_end(process.clk()),
            ExecutionEnd => {
                self.tx_progress.print_stages();
            },
        }

        Ok(HostResponse::None)
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        self.host.on_event(process, event_id)
    }
}

// TRANSACTION PROGRESS
// ================================================================================================

/// Contains the information about the number of cycles for each of the transaction execution
/// stages.
#[derive(Default)]
struct TransactionProgress {
    prologue: CycleInterval,
    notes_processing: CycleInterval,
    note_consuming: Vec<CycleInterval>,
    tx_script_processing: CycleInterval,
    epilogue: CycleInterval,
}

impl TransactionProgress {
    /// Prints out the lengths of cycle intervals for each execution stage.
    pub fn print_stages(&self) {
        println!(
            "Number of cycles it takes to execule:\n- Prologue: {},\n- Notes processing: {},",
            self.prologue
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string()),
            self.notes_processing
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        );

        for (index, note) in self.note_consuming.iter().enumerate() {
            println!(
                "--- Note #{}: {}",
                index,
                note.get_interval_len()
                    .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                    .unwrap_or("invalid interval".to_string())
            )
        }

        println!(
            "- Transaction script processing: {},\n- Epilogue: {}",
            self.tx_script_processing
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string()),
            self.epilogue
                .get_interval_len()
                .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                .unwrap_or("invalid interval".to_string())
        );
    }
}

/// Stores the cycles corresponding to the start and the end of an interval.
#[derive(Default)]
struct CycleInterval {
    start: Option<u32>,
    end: Option<u32>,
}

impl CycleInterval {
    pub fn new(start: u32) -> Self {
        Self { start: Some(start), end: None }
    }

    pub fn set_start(&mut self, s: u32) {
        self.start = Some(s);
    }

    pub fn set_end(&mut self, e: u32) {
        self.end = Some(e);
    }

    /// Calculate the length of the interval
    pub fn get_interval_len(&self) -> Option<u32> {
        if let Some(start) = self.start {
            if let Some(end) = self.end {
                if end >= start {
                    return Some(end - start);
                }
            }
        }
        None
    }
}

// MOCK DATA STORE
// ================================================================================================

#[derive(Clone)]
pub struct MockDataStore {
    pub account: Account,
    pub block_header: BlockHeader,
    pub block_chain: ChainMmr,
    pub notes: Vec<InputNote>,
    pub tx_args: TransactionArgs,
}

impl MockDataStore {
    pub fn new(asset_preservation: AssetPreservationStatus) -> Self {
        let (tx_inputs, tx_args) =
            mock_inputs(MockAccountType::StandardExisting, asset_preservation);
        let (account, _, block_header, block_chain, notes) = tx_inputs.into_parts();

        Self {
            account,
            block_header,
            block_chain,
            notes: notes.into_vec(),
            tx_args,
        }
    }

    pub fn tx_args(&self) -> &TransactionArgs {
        &self.tx_args
    }
}

impl Default for MockDataStore {
    fn default() -> Self {
        Self::new(AssetPreservationStatus::Preserved)
    }
}

impl DataStore for MockDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        assert_eq!(block_num, self.block_header.block_num());
        assert_eq!(notes.len(), self.notes.len());

        let notes = self
            .notes
            .iter()
            .filter(|note| notes.contains(&note.id()))
            .cloned()
            .collect::<Vec<_>>();

        Ok(TransactionInputs::new(
            self.account.clone(),
            None,
            self.block_header,
            self.block_chain.clone(),
            InputNotes::new(notes).unwrap(),
        )
        .unwrap())
    }

    fn get_account_code(&self, account_id: AccountId) -> Result<ModuleAst, DataStoreError> {
        assert_eq!(account_id, self.account.id());
        Ok(self.account.code().module().clone())
    }
}
