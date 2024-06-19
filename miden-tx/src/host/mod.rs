use alloc::{collections::BTreeMap, rc::Rc, string::ToString, vec::Vec};

use miden_lib::transaction::{
    memory::CURRENT_CONSUMED_NOTE_PTR, TransactionEvent, TransactionKernelError, TransactionTrace,
};
use miden_objects::{
    accounts::{AccountDelta, AccountId, AccountStorage, AccountStub},
    assets::Asset,
    notes::NoteId,
    transaction::OutputNote,
    Digest, Hasher,
};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Felt, Host, HostResponse, ProcessState,
};

mod account_delta_tracker;
use account_delta_tracker::AccountDeltaTracker;

mod account_procs;
use account_procs::AccountProcedureIndexMap;

mod note_builder;
use note_builder::OutputNoteBuilder;

mod tx_progress;
pub use tx_progress::TransactionProgress;

use crate::{auth::TransactionAuthenticator, KERNEL_ERRORS};

// CONSTANTS
// ================================================================================================

pub const STORAGE_TREE_DEPTH: Felt = Felt::new(AccountStorage::STORAGE_TREE_DEPTH as u64);

// TRANSACTION HOST
// ================================================================================================

/// Transaction host is responsible for handling [Host] requests made by a transaction kernel.
pub struct TransactionHost<A, T> {
    /// Advice provider which is used to provide non-deterministic inputs to the transaction
    /// runtime.
    adv_provider: A,

    /// Accumulates the state changes notified via events.
    account_delta: AccountDeltaTracker,

    /// A map for the account's procedures.
    acct_procedure_index_map: AccountProcedureIndexMap,

    /// The list of notes created while executing a transaction stored as note_ptr |-> note_builder
    /// map.
    output_notes: BTreeMap<usize, OutputNoteBuilder>,

    /// Provides a way to get a signature for a message into a transaction
    authenticator: Option<Rc<T>>,

    /// Contains the information about the number of cycles for each of the transaction execution
    /// stages.
    tx_progress: TransactionProgress,

    /// Contains generated signatures for messages
    generated_signatures: BTreeMap<Digest, Vec<Felt>>,

    /// Contains mappings from error codes to the related error messages
    error_messages: BTreeMap<u32, &'static str>,
}

impl<A: AdviceProvider, T: TransactionAuthenticator> TransactionHost<A, T> {
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(account: AccountStub, adv_provider: A, authenticator: Option<Rc<T>>) -> Self {
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        let kernel_assertion_errors = BTreeMap::from(KERNEL_ERRORS);
        Self {
            adv_provider,
            account_delta: AccountDeltaTracker::new(&account),
            acct_procedure_index_map: proc_index_map,
            output_notes: BTreeMap::default(),
            authenticator,
            tx_progress: TransactionProgress::default(),
            generated_signatures: BTreeMap::new(),
            error_messages: kernel_assertion_errors,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (A, AccountDelta, Vec<OutputNote>, BTreeMap<Digest, Vec<Felt>>) {
        let output_notes = self.output_notes.into_values().map(|builder| builder.build()).collect();

        (
            self.adv_provider,
            self.account_delta.into_delta(),
            output_notes,
            self.generated_signatures,
        )
    }

    /// Returns a reference to the `tx_progress` field of the [`TransactionHost`].
    pub fn tx_progress(&self) -> &TransactionProgress {
        &self.tx_progress
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Crates a new [OutputNoteBuilder] from the data on the operand stack and stores it into the
    /// `output_notes` field of this [TransactionHost].
    ///
    /// Expected stack state: `[aux, note_type, sender_acct_id, tag, note_ptr, RECIPIENT, ...]`
    fn on_note_after_created<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let stack = process.get_stack_state();
        // # => [aux, note_type, sender_acct_id, tag, note_ptr, RECIPIENT, note_idx]

        let note_idx: usize = stack[9].as_int() as usize;

        assert_eq!(note_idx, self.output_notes.len(), "note index mismatch");

        let note_builder = OutputNoteBuilder::new(stack, &self.adv_provider)?;

        self.output_notes.insert(note_idx, note_builder);

        Ok(())
    }

    /// Adds an asset at the top of the [OutputNoteBuilder] identified by the note pointer.
    ///
    /// Expected stack state: [ASSET, note_ptr, ...]
    fn on_note_before_add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let stack = process.get_stack_state();
        //# => [ASSET, note_ptr, num_of_assets, note_idx]

        let note_idx = stack[6].as_int();
        assert!(note_idx < self.output_notes.len() as u64);
        let node_idx = note_idx as usize;

        let asset = Asset::try_from(process.get_stack_word(0))
            .map_err(TransactionKernelError::MalformedAsset)?;

        let note_builder = self
            .output_notes
            .get_mut(&node_idx)
            .ok_or_else(|| TransactionKernelError::MissingNote(format!("{:?}", &note_idx)))?;

        note_builder.add_asset(asset)?;

        Ok(())
    }

    /// Loads the index of the procedure root onto the advice stack.
    ///
    /// Expected stack state: [PROC_ROOT, ...]
    fn on_account_push_procedure_index<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let proc_idx = self.acct_procedure_index_map.get_proc_index(process)?;
        self.adv_provider
            .push_stack(AdviceSource::Value(proc_idx.into()))
            .expect("failed to push value onto advice stack");
        Ok(())
    }

    /// Extracts the nonce increment from the process state and adds it to the nonce delta tracker.
    ///
    /// Expected stack state: [nonce_delta, ...]
    pub fn on_account_before_increment_nonce<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let value = process.get_stack_item(0);
        self.account_delta.increment_nonce(value);
        Ok(())
    }

    // ACCOUNT STORAGE UPDATE HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Extracts information from the process state about the storage slot being updated and
    /// records the latest value of this storage slot.
    ///
    /// Expected stack state: [slot_index, NEW_SLOT_VALUE, CURRENT_SLOT_VALUE, ...]
    pub fn on_account_storage_after_set_item<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        // get slot index from the stack and make sure it is valid
        let slot_index = process.get_stack_item(0);
        if slot_index.as_int() as usize >= AccountStorage::NUM_STORAGE_SLOTS {
            return Err(TransactionKernelError::InvalidStorageSlotIndex(slot_index.as_int()));
        }

        // get the value to which the slot is being updated
        let new_slot_value = [
            process.get_stack_item(4),
            process.get_stack_item(3),
            process.get_stack_item(2),
            process.get_stack_item(1),
        ];

        // get the current value for the slot
        let current_slot_value = [
            process.get_stack_item(8),
            process.get_stack_item(7),
            process.get_stack_item(6),
            process.get_stack_item(5),
        ];

        // update the delta tracker only if the current and new values are different
        if current_slot_value != new_slot_value {
            let slot_index = slot_index.as_int() as u8;
            self.account_delta.storage_tracker().slot_update(slot_index, new_slot_value);
        }

        Ok(())
    }

    /// Extracts information from the process state about the storage map being updated and
    /// records the latest values of this storage map.
    ///
    /// Expected stack state: [slot_index, NEW_MAP_KEY, NEW_MAP_VALUE, ...]
    pub fn on_account_storage_after_set_map_item<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        // get slot index from the stack and make sure it is valid
        let slot_index = process.get_stack_item(0);
        if slot_index.as_int() as usize >= AccountStorage::NUM_STORAGE_SLOTS {
            return Err(TransactionKernelError::InvalidStorageSlotIndex(slot_index.as_int()));
        }

        // get the KEY to which the slot is being updated
        let new_map_key = [
            process.get_stack_item(4),
            process.get_stack_item(3),
            process.get_stack_item(2),
            process.get_stack_item(1),
        ];

        // get the VALUE to which the slot is being updated
        let new_map_value = [
            process.get_stack_item(8),
            process.get_stack_item(7),
            process.get_stack_item(6),
            process.get_stack_item(5),
        ];

        let slot_index = slot_index.as_int() as u8;
        self.account_delta
            .storage_tracker()
            .maps_update(slot_index, new_map_key, new_map_value);

        Ok(())
    }

    // ACCOUNT VAULT UPDATE HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Extracts the asset that is being added to the account's vault from the process state and
    /// updates the appropriate fungible or non-fungible asset map.
    ///
    /// Expected stack state: [ASSET, ...]
    pub fn on_account_vault_after_add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let asset: Asset = process
            .get_stack_word(0)
            .try_into()
            .map_err(TransactionKernelError::MalformedAssetOnAccountVaultUpdate)?;

        self.account_delta.vault_tracker().add_asset(asset);
        Ok(())
    }

    /// Extracts the asset that is being removed from the account's vault from the process state
    /// and updates the appropriate fungible or non-fungible asset map.
    ///
    /// Expected stack state: [ASSET, ...]
    pub fn on_account_vault_after_remove_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let asset: Asset = process
            .get_stack_word(0)
            .try_into()
            .map_err(TransactionKernelError::MalformedAssetOnAccountVaultUpdate)?;

        self.account_delta.vault_tracker().remove_asset(asset);
        Ok(())
    }

    // ADVICE INJECTOR HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Returns a signature as a response to the `SigToStack` injector.
    ///
    /// This signature is created during transaction execution and stored for use as advice map
    /// inputs in the proving host. If not already present in the advice map, it is requested from
    /// the host's authenticator.
    pub fn on_signature_requested<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<HostResponse, ExecutionError> {
        let pub_key = process.get_stack_word(0);
        let msg = process.get_stack_word(1);
        let signature_key = Hasher::merge(&[pub_key.into(), msg.into()]);

        let signature = if let Some(signature) = self.adv_provider.get_mapped_values(&signature_key)
        {
            signature.to_vec()
        } else {
            let account_delta = self.account_delta.clone().into_delta();

            let signature: Vec<Felt> = match &self.authenticator {
                None => Err(ExecutionError::FailedSignatureGeneration(
                    "No authenticator assigned to transaction host",
                )),
                Some(authenticator) => {
                    authenticator.get_signature(pub_key, msg, &account_delta).map_err(|_| {
                        ExecutionError::FailedSignatureGeneration("Error generating signature")
                    })
                },
            }?;

            self.generated_signatures.insert(signature_key, signature.clone());
            signature
        };

        for r in signature {
            self.adv_provider.push_stack(AdviceSource::Value(r))?;
        }

        Ok(HostResponse::None)
    }

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the currently executing input note, or None if the note execution hasn't
    /// started yet or has already ended.
    ///
    /// # Errors
    /// Returns an error if the address of the currently executing input note is invalid (e.g.,
    /// greater than `u32::MAX`).
    fn get_current_note_id<S: ProcessState>(process: &S) -> Result<Option<NoteId>, ExecutionError> {
        // get the word where note address is stored
        let note_address_word = process.get_mem_value(process.ctx(), CURRENT_CONSUMED_NOTE_PTR);
        // get the note address in `Felt` from or return `None` if the address hasn't been accessed
        // previously.
        let note_address_felt = match note_address_word {
            Some(w) => w[0],
            None => return Ok(None),
        };
        // get the note address
        let note_address: u32 = note_address_felt
            .try_into()
            .map_err(|_| ExecutionError::MemoryAddressOutOfBounds(note_address_felt.as_int()))?;
        // if `note_address` == 0 note execution has ended and there is no valid note address
        if note_address == 0 {
            Ok(None)
        } else {
            Ok(process.get_mem_value(process.ctx(), note_address).map(NoteId::from))
        }
    }
}

impl<A: AdviceProvider, T: TransactionAuthenticator> Host for TransactionHost<A, T> {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.get_advice(process, &extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        match injector {
            AdviceInjector::SigToStack { .. } => self.on_signature_requested(process),
            injector => self.adv_provider.set_advice(process, &injector),
        }
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        let event = TransactionEvent::try_from(event_id)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(format!(
                "{event} event can only be emitted from the root context"
            )));
        }

        match event {
            TransactionEvent::AccountVaultBeforeAddAsset => Ok(()),
            TransactionEvent::AccountVaultAfterAddAsset => {
                self.on_account_vault_after_add_asset(process)
            },

            TransactionEvent::AccountVaultBeforeRemoveAsset => Ok(()),
            TransactionEvent::AccountVaultAfterRemoveAsset => {
                self.on_account_vault_after_remove_asset(process)
            },

            TransactionEvent::AccountStorageBeforeSetItem => Ok(()),
            TransactionEvent::AccountStorageAfterSetItem => {
                self.on_account_storage_after_set_item(process)
            },

            TransactionEvent::AccountStorageBeforeSetMapItem => Ok(()),
            TransactionEvent::AccountStorageAfterSetMapItem => {
                self.on_account_storage_after_set_map_item(process)
            },

            TransactionEvent::AccountBeforeIncrementNonce => {
                self.on_account_before_increment_nonce(process)
            },
            TransactionEvent::AccountAfterIncrementNonce => Ok(()),

            TransactionEvent::AccountPushProcedureIndex => {
                self.on_account_push_procedure_index(process)
            },

            TransactionEvent::NoteBeforeCreated => Ok(()),
            TransactionEvent::NoteAfterCreated => self.on_note_after_created(process),

            TransactionEvent::NoteBeforeAddAsset => self.on_note_before_add_asset(process),
            TransactionEvent::NoteAfterAddAsset => Ok(()),
        }
        .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        Ok(HostResponse::None)
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
            PrologueStart => self.tx_progress.start_prologue(process.clk()),
            PrologueEnd => self.tx_progress.end_prologue(process.clk()),
            NotesProcessingStart => self.tx_progress.start_notes_processing(process.clk()),
            NotesProcessingEnd => self.tx_progress.end_notes_processing(process.clk()),
            NoteExecutionStart => {
                let note_id = Self::get_current_note_id(process)?
                    .expect("Note execution interval measurement is incorrect: check the placement of the start and the end of the interval");
                self.tx_progress.start_note_execution(process.clk(), note_id);
            },
            NoteExecutionEnd => self.tx_progress.end_note_execution(process.clk()),
            TxScriptProcessingStart => self.tx_progress.start_tx_script_processing(process.clk()),
            TxScriptProcessingEnd => self.tx_progress.end_tx_script_processing(process.clk()),
            EpilogueStart => self.tx_progress.start_epilogue(process.clk()),
            EpilogueEnd => self.tx_progress.end_epilogue(process.clk()),
        }

        Ok(HostResponse::None)
    }

    fn on_assert_failed<S: ProcessState>(&mut self, process: &S, err_code: u32) -> ExecutionError {
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
}
