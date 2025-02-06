use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::ToString,
    sync::Arc,
    vec::Vec,
};

use miden_lib::{
    errors::tx_kernel_errors::TX_KERNEL_ERRORS,
    transaction::{
        memory::CURRENT_INPUT_NOTE_PTR, AccountProcedureIndexMap, OutputNoteBuilder,
        TransactionTrace,
    },
    utils::sync::RwLock,
    AccountDeltaTracker, EventHandlersInputs, MidenFalconSigner, MidenFalconSignerInputs, MidenLib,
    StdLibrary, TransactionAuthenticator,
};
use miden_objects::{
    account::{AccountDelta, AccountHeader},
    note::NoteId,
    transaction::{OutputNote, TransactionMeasurements},
    vm::RowIndex,
    Digest,
};
use vm_core::{DebugOptions, Felt};
use vm_processor::{
    AdviceProvider, EventHandlerRegistry, ExecutionError, Host, HostLibrary, MastForest,
    MastForestStore, ProcessState,
};

mod tx_progress;
pub use tx_progress::TransactionProgress;

use crate::{errors::TransactionHostError, executor::TransactionMastStore};

// TRANSACTION HOST
// ================================================================================================

/// Transaction host is responsible for handling [Host] requests made by a transaction kernel.
///
/// Transaction hosts are created on a per-transaction basis. That is, a transaction host is meant
/// to support execution of a single transaction and is discarded after the transaction finishes
/// execution.
pub struct TransactionHost<A> {
    /// Advice provider which is used to provide non-deterministic inputs to the transaction
    /// runtime.
    adv_provider: A,

    event_registry: EventHandlerRegistry<A>,

    /// MAST store which contains the code required to execute the transaction.
    mast_store: Arc<TransactionMastStore>,

    /// Account state changes accumulated during transaction execution.
    ///
    /// This field is updated by the [TransactionHost::on_event()] handler.
    account_delta: Arc<RwLock<AccountDeltaTracker>>,

    /// The list of notes created while executing a transaction stored as note_ptr |-> note_builder
    /// map.
    output_notes: Arc<RwLock<BTreeMap<usize, OutputNoteBuilder>>>,

    /// Contains previously generated signatures (as a message |-> signature map) required for
    /// transaction execution.
    ///
    /// If a required signature is not present in this map, the host will attempt to generate the
    /// signature using the transaction authenticator.
    generated_signatures: BTreeMap<Digest, Vec<Felt>>,

    /// Tracks the number of cycles for each of the transaction execution stages.
    ///
    /// This field is updated by the [TransactionHost::on_trace()] handler.
    tx_progress: TransactionProgress,

    /// Contains mappings from error codes to the related error messages.
    ///
    /// This map is initialized at construction time from the [`TX_KERNEL_ERRORS`] array.
    error_messages: BTreeMap<u32, &'static str>,
}

impl<A> TransactionHost<A>
where
    A: AdviceProvider + Default + 'static,
{
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(
        account: AccountHeader,
        adv_provider: A,
        mast_store: Arc<TransactionMastStore>,
        authenticator: Option<Arc<dyn TransactionAuthenticator>>,
        mut account_code_commitments: BTreeSet<Digest>,
    ) -> Result<Self, TransactionHostError> {
        // currently, the executor/prover do not keep track of the code commitment of the native
        // account, so we add it to the set here
        account_code_commitments.insert(account.code_commitment());

        let proc_index_map =
            AccountProcedureIndexMap::new(account_code_commitments.clone(), &adv_provider)
                .map_err(TransactionHostError::AccountProcedureIndexMapError)?;
        let account_delta = Arc::new(RwLock::new(AccountDeltaTracker::new(&account)));
        let output_notes = Arc::new(RwLock::new(BTreeMap::default()));

        let event_registry = {
            // Miden library event handlers
            let midenlib_handlers = {
                // TODO(plafer): `account` and others can probably be passed by ref
                let miden_lib_inputs = EventHandlersInputs {
                    account: account.clone(),
                    account_delta: account_delta.clone(),
                    account_code_commitments,
                    account_proc_index_map: proc_index_map.clone(),
                    output_notes: output_notes.clone(),
                };

                MidenLib::default().get_event_handlers(miden_lib_inputs)
            };

            // Standard library event handlers
            let stdlib_handlers = {
                let signer_inputs = MidenFalconSignerInputs {
                    account_delta: account_delta.clone(),
                    authenticator: authenticator.clone(),
                };

                StdLibrary::<MidenFalconSigner, MidenFalconSignerInputs>::new()
                    .get_event_handlers(signer_inputs)
            };

            // Build event registry and return
            let mut event_registry = EventHandlerRegistry::default();
            // TODO(plafer): add a variant to `TransactionHostError`
            event_registry
                .register_event_handlers(midenlib_handlers.into_iter().chain(stdlib_handlers))
                .expect("event handlers registration failed");

            event_registry
        };

        let kernel_assertion_errors = BTreeMap::from(TX_KERNEL_ERRORS);
        Ok(Self {
            adv_provider,
            event_registry,
            mast_store,
            account_delta,
            output_notes,
            tx_progress: TransactionProgress::default(),
            generated_signatures: BTreeMap::new(),
            error_messages: kernel_assertion_errors,
        })
    }

    /// Consumes `self` and returns the advice provider, account delta, output notes, generated
    /// signatures, and transaction progress.
    pub fn into_parts(
        self,
    ) -> (
        A,
        AccountDelta,
        Vec<OutputNote>,
        BTreeMap<Digest, Vec<Felt>>,
        TransactionProgress,
    ) {
        // TODO(plafer): avoid the clone
        let output_notes = self
            .output_notes
            .read()
            .clone()
            .into_values()
            .map(|builder| builder.build())
            .collect();

        (
            self.adv_provider,
            // TODO(plafer): avoid the clone
            self.account_delta.read().clone().into_delta(),
            output_notes,
            self.generated_signatures,
            self.tx_progress,
        )
    }

    /// Returns a reference to the `tx_progress` field of this transaction host.
    pub fn tx_progress(&self) -> &TransactionProgress {
        &self.tx_progress
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the currently executing input note, or None if the note execution hasn't
    /// started yet or has already ended.
    ///
    /// # Errors
    /// Returns an error if the address of the currently executing input note is invalid (e.g.,
    /// greater than `u32::MAX`).
    fn get_current_note_id(process: ProcessState) -> Result<Option<NoteId>, ExecutionError> {
        // get the note address in `Felt` or return `None` if the address hasn't been accessed
        // previously.
        let note_address_felt = match process.get_mem_value(process.ctx(), CURRENT_INPUT_NOTE_PTR) {
            Some(addr) => addr,
            None => return Ok(None),
        };
        // convert note address into u32
        let note_address: u32 = note_address_felt
            .try_into()
            .map_err(|_| ExecutionError::MemoryAddressOutOfBounds(note_address_felt.as_int()))?;
        // if `note_address` == 0 note execution has ended and there is no valid note address
        if note_address == 0 {
            Ok(None)
        } else {
            Ok(process.get_mem_word(process.ctx(), note_address)?.map(NoteId::from))
        }
    }
}

// HOST IMPLEMENTATION FOR TRANSACTION HOST
// ================================================================================================

impl<A> Host for TransactionHost<A>
where
    A: AdviceProvider + Default + 'static,
{
    type AdviceProvider = A;

    fn advice_provider(&self) -> &Self::AdviceProvider {
        &self.adv_provider
    }

    fn advice_provider_mut(&mut self) -> &mut Self::AdviceProvider {
        &mut self.adv_provider
    }

    fn get_mast_forest(&self, node_digest: &Digest) -> Option<Arc<MastForest>> {
        self.mast_store.get(node_digest)
    }

    fn on_event(&mut self, process: ProcessState, event_id: u32) -> Result<(), ExecutionError> {
        let handler = self
            .event_registry
            .get_event_handler(event_id)
            .ok_or_else(|| ExecutionError::EventHandlerNotFound { event_id, clk: process.clk() })?;

        handler
            .on_event(process, &mut self.adv_provider)
            .map_err(ExecutionError::EventError)
    }

    fn on_trace(&mut self, process: ProcessState, trace_id: u32) -> Result<(), ExecutionError> {
        let event = TransactionTrace::try_from(trace_id)
            .map_err(|err| ExecutionError::EventError(Box::new(err)))?;

        use TransactionTrace::*;
        match event {
            PrologueStart => self.tx_progress.start_prologue(process.clk()),
            PrologueEnd => self.tx_progress.end_prologue(process.clk()),
            NotesProcessingStart => self.tx_progress.start_notes_processing(process.clk()),
            NotesProcessingEnd => self.tx_progress.end_notes_processing(process.clk()),
            NoteExecutionStart => {
                let note_id = Self::get_current_note_id(process)?.expect(
                    "Note execution interval measurement is incorrect: check the placement of the start and the end of the interval",
                );
                self.tx_progress.start_note_execution(process.clk(), note_id);
            },
            NoteExecutionEnd => self.tx_progress.end_note_execution(process.clk()),
            TxScriptProcessingStart => self.tx_progress.start_tx_script_processing(process.clk()),
            TxScriptProcessingEnd => self.tx_progress.end_tx_script_processing(process.clk()),
            EpilogueStart => self.tx_progress.start_epilogue(process.clk()),
            EpilogueEnd => self.tx_progress.end_epilogue(process.clk()),
        }

        Ok(())
    }

    fn on_assert_failed(&mut self, process: ProcessState, err_code: u32) -> ExecutionError {
        let err_msg = self
            .error_messages
            .get(&err_code)
            .map_or("Unknown error".to_string(), |msg| msg.to_string());
        ExecutionError::FailedAssertion {
            clk: process.clk(),
            err_code,
            err_msg: Some(err_msg),
        }
    }

    fn on_debug(
        &mut self,
        _process: ProcessState,
        _options: &DebugOptions,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }
}
