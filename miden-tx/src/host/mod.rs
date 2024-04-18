use alloc::{collections::BTreeMap, string::ToString, vec::Vec};

use miden_lib::transaction::{
    memory::{ACCT_STORAGE_ROOT_PTR, CURRENT_CONSUMED_NOTE_PTR},
    TransactionEvent, TransactionKernelError, TransactionTrace,
};
use miden_objects::{
    accounts::{AccountDelta, AccountId, AccountStorage, AccountStub},
    assets::Asset,
    notes::{
        Note, NoteAssets, NoteEnvelope, NoteId, NoteInputs, NoteMetadata, NoteRecipient,
        NoteScript, NoteTag, NoteType,
    },
    transaction::OutputNote,
    Digest,
};
use vm_processor::{
    crypto::NodeIndex, AdviceExtractor, AdviceInjector, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Felt, Host, HostResponse, ProcessState,
};

mod account_delta_tracker;
use account_delta_tracker::AccountDeltaTracker;

mod account_procs;
use account_procs::AccountProcedureIndexMap;

mod tx_progress;
pub use tx_progress::TransactionProgress;

// CONSTANTS
// ================================================================================================

pub const STORAGE_TREE_DEPTH: Felt = Felt::new(AccountStorage::STORAGE_TREE_DEPTH as u64);

// TRANSACTION HOST
// ================================================================================================

/// Transaction host is responsible for handling [Host] requests made by a transaction kernel.
pub struct TransactionHost<A> {
    /// Advice provider which is used to provide non-deterministic inputs to the transaction
    /// runtime.
    adv_provider: A,

    /// Accumulates the state changes notified via events.
    account_delta: AccountDeltaTracker,

    /// A map for the account's procedures.
    acct_procedure_index_map: AccountProcedureIndexMap,

    /// The list of notes created while executing a transaction.
    output_notes: Vec<OutputNote>,

    /// Contains the information about the number of cycles for each of the transaction execution
    /// stages.
    tx_progress: TransactionProgress,
}

impl<A: AdviceProvider> TransactionHost<A> {
    /// Returns a new [TransactionHost] instance with the provided [AdviceProvider].
    pub fn new(account: AccountStub, adv_provider: A) -> Self {
        let proc_index_map = AccountProcedureIndexMap::new(account.code_root(), &adv_provider);
        Self {
            adv_provider,
            account_delta: AccountDeltaTracker::new(&account),
            acct_procedure_index_map: proc_index_map,
            output_notes: Vec::new(),
            tx_progress: TransactionProgress::default(),
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (A, AccountDelta, Vec<OutputNote>) {
        (self.adv_provider, self.account_delta.into_delta(), self.output_notes)
    }

    /// Returns a reference to the `tx_progress` field of the [`TransactionHost`].
    pub fn tx_progress(&self) -> &TransactionProgress {
        &self.tx_progress
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_note_created<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let stack = process.get_stack_state();

        // Stack:
        // # => [aux, note_type, sender_acct_id, tag, note_ptr, ASSET, RECIPIENT]
        let aux = stack[0];
        let note_type =
            NoteType::try_from(stack[1]).map_err(TransactionKernelError::MalformedNoteType)?;
        let sender =
            AccountId::try_from(stack[2]).map_err(TransactionKernelError::MalformedAccountId)?;
        let tag = NoteTag::try_from(stack[3])
            .map_err(|_| TransactionKernelError::MalformedTag(stack[3]))?;
        let asset = Asset::try_from([stack[8], stack[7], stack[6], stack[5]])
            .map_err(TransactionKernelError::MalformedAsset)?;
        let recipient = Digest::new([stack[12], stack[11], stack[10], stack[9]]);
        let vault =
            NoteAssets::new(vec![asset]).map_err(TransactionKernelError::MalformedNoteType)?;

        let metadata = NoteMetadata::new(sender, note_type, tag, aux)
            .map_err(TransactionKernelError::MalformedNoteMetadata)?;

        let note = if metadata.note_type() == NoteType::Public {
            let data = self.adv_provider.get_mapped_values(&recipient).ok_or(
                TransactionKernelError::MissingNoteDetails(metadata, vault.clone(), recipient),
            )?;
            if data.len() != 12 {
                return Err(TransactionKernelError::MalformedRecipientData(data.to_vec()));
            }
            let inputs_hash = Digest::new([data[0], data[1], data[2], data[3]]);
            let inputs_key = NoteInputs::commitment_to_key(inputs_hash);
            let script_hash = Digest::new([data[4], data[5], data[6], data[7]]);
            let serial_num = [data[8], data[9], data[10], data[11]];
            let input_els = self.adv_provider.get_mapped_values(&inputs_key);
            let script_data = self.adv_provider.get_mapped_values(&script_hash).unwrap_or(&[]);

            let inputs = NoteInputs::new(input_els.map(|e| e.to_vec()).unwrap_or_default())
                .map_err(TransactionKernelError::MalformedNoteInputs)?;

            let script = NoteScript::try_from(script_data)
                .map_err(|_| TransactionKernelError::MalformedNoteScript(script_data.to_vec()))?;
            let recipient = NoteRecipient::new(serial_num, script, inputs);
            OutputNote::Public(Note::new(vault, metadata, recipient))
        } else {
            let note_id = NoteId::new(recipient, vault.commitment());
            OutputNote::Private(
                NoteEnvelope::new(note_id, metadata).expect("NoteType checked above"),
            )
        };

        self.output_notes.push(note);

        Ok(())
    }

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
    pub fn on_account_increment_nonce<S: ProcessState>(
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
    pub fn on_account_storage_set_item<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let storage_root = process
            .get_mem_value(ContextId::root(), ACCT_STORAGE_ROOT_PTR)
            .expect("no storage root");

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

        // try to get the current value for the slot from the advice provider
        let current_slot_value = self
            .adv_provider
            .get_tree_node(storage_root, &STORAGE_TREE_DEPTH, &slot_index)
            .map_err(|err| {
                TransactionKernelError::MissingStorageSlotValue(
                    slot_index.as_int() as u8,
                    err.to_string(),
                )
            })?;

        // update the delta tracker only if the current and new values are different
        if current_slot_value != new_slot_value {
            let slot_index = slot_index.as_int() as u8;
            self.account_delta.storage_tracker().slot_update(slot_index, new_slot_value);
        }

        Ok(())
    }

    // ACCOUNT VAULT UPDATE HANDLERS
    // --------------------------------------------------------------------------------------------

    /// Extracts the asset that is being added to the account's vault from the process state and
    /// updates the appropriate fungible or non-fungible asset map.
    pub fn on_account_vault_add_asset<S: ProcessState>(
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
    pub fn on_account_vault_remove_asset<S: ProcessState>(
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

    // HELPER FUNCTIONS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the consumed note being executed.
    fn get_current_note_id<S: ProcessState>(process: &S) -> Result<Option<NoteId>, ExecutionError> {
        let note_address_felt = process
            .get_mem_value(process.ctx(), CURRENT_CONSUMED_NOTE_PTR)
            .expect("current consumed note pointer invalid")[0];
        let note_address: u32 = note_address_felt
            .try_into()
            .map_err(|_| ExecutionError::MemoryAddressOutOfBounds(note_address_felt.as_int()))?;
        Ok(process.get_mem_value(process.ctx(), note_address).map(NoteId::from))
    }
}

impl<A: AdviceProvider> Host for TransactionHost<A> {
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
        self.adv_provider.set_advice(process, &injector)
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
            TransactionEvent::AccountVaultAddAsset => self.on_account_vault_add_asset(process),
            TransactionEvent::AccountVaultRemoveAsset => {
                self.on_account_vault_remove_asset(process)
            },
            TransactionEvent::AccountStorageSetItem => self.on_account_storage_set_item(process),
            TransactionEvent::AccountIncrementNonce => self.on_account_increment_nonce(process),
            TransactionEvent::AccountPushProcedureIndex => {
                self.on_account_push_procedure_index(process)
            },
            TransactionEvent::NoteCreated => self.on_note_created(process),
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
                let note_id = Self::get_current_note_id(process)?;
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
}
