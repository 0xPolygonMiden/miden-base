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

const END_OF_PROLOGUE: u8 = 0;
const END_OF_NODE_PROCESSING: u8 = 1;
const END_OF_TX_SCRIPT_PROCESSING: u8 = 2;
const END_OF_EPILOGUE: u8 = 3;

// BENCHMARK HOST
// ================================================================================================

/// Wrapper around [TransactionHost] used for benchmarking
pub struct BenchHost<A: AdviceProvider> {
    host: TransactionHost<A>,
    prologue: u32,
    note_processing: u32,
    tx_script_processing: u32,
    epilogue: u32,
}

impl<A: AdviceProvider> BenchHost<A> {
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        Self {
            host: TransactionHost::new(account, adv_provider),
            prologue: 0,
            note_processing: 0,
            tx_script_processing: 0,
            epilogue: 0,
        }
    }

    /// Calculate the final amount of cycles spent for each stage
    fn print_stages_cycle(&self) {
        std::println!(
            "Number of cycles it takes to execule:
    - Prologue: {},
    - Note processing: {},
    - Transaction script processing: {},
    - Epilogue: {}
    ",
            self.prologue - 2,
            self.note_processing - self.prologue - 2,
            self.tx_script_processing - self.note_processing - 2,
            self.epilogue - self.tx_script_processing - 2
        )
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
        match trace_id as u8 {
            END_OF_PROLOGUE => self.prologue = process.clk(),
            END_OF_NODE_PROCESSING => self.note_processing = process.clk(),
            END_OF_TX_SCRIPT_PROCESSING => self.tx_script_processing = process.clk(),
            END_OF_EPILOGUE => {
                self.epilogue = process.clk();
                self.print_stages_cycle()
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
