use crate::MockDataStore;
use miden_lib::transaction::{ToTransactionKernelInputs, TransactionTrace};
use miden_objects::{accounts::AccountStub, transaction::TransactionArgs};
use miden_tx::{TransactionExecutor, TransactionHost};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, ExecutionError, ExecutionOptions, Host,
    HostResponse, ProcessState, RecAdviceProvider,
};

// CONSTANTS
// ================================================================================================

/// Number of cycles needed to create an empty span whithout changing the stack state.
const SPAN_CREATION_SHIFT: u32 = 2;

// BENCHMARK HOST
// ================================================================================================

/// Wrapper around [TransactionHost] used for benchmarking.
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
                #[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
    pub fn print_stages(&self) {
        std::println!(
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
            std::println!(
                "--- Note #{}: {}",
                index,
                note.get_interval_len()
                    .map(|len| (len - SPAN_CREATION_SHIFT).to_string())
                    .unwrap_or("invalid interval".to_string())
            )
        }

        std::println!(
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

    /// Calculate the length of the described interval
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

// BENCHMARKS
// ================================================================================================

#[ignore]
#[test]
fn benchmark_default_tx() {
    let data_store = MockDataStore::default();
    let mut executor = TransactionExecutor::new(data_store.clone()).with_tracing();

    let account_id = data_store.account.id();
    executor.load_account(account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let transaction = executor
        .prepare_transaction(account_id, block_ref, &note_ids, TransactionArgs::default())
        .unwrap();

    let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
    let advice_recorder: RecAdviceProvider = advice_inputs.into();
    let mut host = BenchHost::new(transaction.account().into(), advice_recorder);

    vm_processor::execute(
        transaction.program(),
        stack_inputs,
        &mut host,
        ExecutionOptions::default().with_tracing(),
    )
    .unwrap();
}
