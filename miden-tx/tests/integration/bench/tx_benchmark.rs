use crate::MockDataStore;
use miden_lib::transaction::ToTransactionKernelInputs;
use miden_objects::{accounts::AccountStub, transaction::TransactionArgs};
use miden_tx::{TransactionExecutor, TransactionHost};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, ExecutionError, ExecutionOptions, Host,
    HostResponse, ProcessState, RecAdviceProvider,
};

// CONSTANTS
// ================================================================================================

const PROLOGUE_START: u32 = 0x2_0000; // 131072
const PROLOGUE_END: u32 = 0x2_0001; // 131073

const NOTE_PROCESSING_START: u32 = 0x2_0002; // 131074
const NOTE_PROCESSING_END: u32 = 0x2_0003; // 131075

const TX_SCRIPT_PROCESSING_START: u32 = 0x2_0004; // 131076
const TX_SCRIPT_PROCESSING_END: u32 = 0x2_0005; // 131077

const EPILOGUE_START: u32 = 0x2_0006; // 131078
const EPILOGUE_END: u32 = 0x2_0007; // 131079

const SPAN_CREATION_SHIFT: u32 = 2;

// BENCHMARK HOST
// ================================================================================================

/// Wrapper around [TransactionHost] used for benchmarking
pub struct BenchHost<A: AdviceProvider> {
    host: TransactionHost<A>,
    prologue_start: Option<u32>,
    note_processing_start: Option<u32>,
    tx_script_processing_start: Option<u32>,
    epilogue_start: Option<u32>,
}

impl<A: AdviceProvider> BenchHost<A> {
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        Self {
            host: TransactionHost::new(account, adv_provider),
            prologue_start: None,
            note_processing_start: None,
            tx_script_processing_start: None,
            epilogue_start: None,
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
        #[cfg(feature = "std")]
        match trace_id {
            PROLOGUE_START => self.prologue_start = Some(process.clk()),
            PROLOGUE_END => {
                if let Some(prologue_start) = self.prologue_start {
                    std::println!(
                        "Number of cycles it takes to execute the prologue: {}",
                        process.clk() - prologue_start - SPAN_CREATION_SHIFT
                    )
                }
            },
            NOTE_PROCESSING_START => self.note_processing_start = Some(process.clk()),
            NOTE_PROCESSING_END => {
                if let Some(note_processing_start) = self.note_processing_start {
                    std::println!(
                        "Number of cycles it takes to execute the note processing: {}",
                        process.clk() - note_processing_start - SPAN_CREATION_SHIFT
                    )
                }
            },
            TX_SCRIPT_PROCESSING_START => self.tx_script_processing_start = Some(process.clk()),
            TX_SCRIPT_PROCESSING_END => {
                if let Some(tx_script_processing_start) = self.tx_script_processing_start {
                    std::println!(
                        "Number of cycles it takes to execute the transaction script processing: {}", 
                        process.clk() - tx_script_processing_start - SPAN_CREATION_SHIFT
                    )
                }
            },
            EPILOGUE_START => self.epilogue_start = Some(process.clk()),
            EPILOGUE_END => {
                if let Some(epilogue_start) = self.epilogue_start {
                    std::println!(
                        "Number of cycles it takes to execute the epilogue: {}",
                        process.clk() - epilogue_start - SPAN_CREATION_SHIFT
                    )
                }
            },
            _ => println!("Invalid trace id was used: {}", trace_id),
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
