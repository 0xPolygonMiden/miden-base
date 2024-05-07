use alloc::{collections::BTreeMap, rc::Rc, string::ToString, vec::Vec};

use miden_lib::transaction::{
    memory::{MemoryAddress, ACCT_STORAGE_ROOT_PTR, CURRENT_CONSUMED_NOTE_PTR},
    TransactionEvent, TransactionKernelError, TransactionTrace,
};
use miden_objects::{
    accounts::{AccountDelta, AccountId, AccountStorage, AccountStub},
    assets::Asset,
    notes::{NoteId, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType},
    transaction::OutputNote,
    Digest, Hasher,
};
use vm_processor::{
    crypto::NodeIndex, AdviceExtractor, AdviceInjector, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Felt, Host, HostResponse, ProcessState,
};

mod account_delta_tracker;
use account_delta_tracker::AccountDeltaTracker;

mod account_procs;
use account_procs::AccountProcedureIndexMap;

mod note_builder;
use note_builder::OutputNoteBuilder;

mod tx_authenticator;
pub use tx_authenticator::{BasicAuthenticator, TransactionAuthenticator};

mod tx_progress;
pub use tx_progress::TransactionProgress;

use crate::KERNEL_ERRORS;

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
    output_notes: BTreeMap<MemoryAddress, OutputNoteBuilder>,

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

    fn on_note_created<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        let stack = process.get_stack_state();

        // Stack:
        // # => [aux, note_type, sender_acct_id, tag, note_ptr, RECIPIENT]
        let aux = stack[0];
        let note_type =
            NoteType::try_from(stack[1]).map_err(TransactionKernelError::MalformedNoteType)?;
        let sender =
            AccountId::try_from(stack[2]).map_err(TransactionKernelError::MalformedAccountId)?;
        let tag = NoteTag::try_from(stack[3])
            .map_err(|_| TransactionKernelError::MalformedTag(stack[3]))?;
        let note_ptr: MemoryAddress =
            stack[4].try_into().map_err(TransactionKernelError::MalformedNotePointer)?;
        let recipient_digest = Digest::new([stack[8], stack[7], stack[6], stack[5]]);

        let metadata = NoteMetadata::new(sender, note_type, tag, aux)
            .map_err(TransactionKernelError::MalformedNoteMetadata)?;

        let note_builder = if let Some(data) =
            self.adv_provider.get_mapped_values(&recipient_digest)
        {
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

            OutputNoteBuilder::with_recipient(metadata, recipient)
        } else {
            OutputNoteBuilder::new(metadata, recipient_digest)?
        };

        self.output_notes.insert(note_ptr, note_builder);

        Ok(())
    }

    fn on_note_add_asset<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), TransactionKernelError> {
        //# => [ASSET, note_ptr]
        let note_ptr: MemoryAddress = process
            .get_stack_item(4)
            .try_into()
            .map_err(TransactionKernelError::MalformedNotePointer)?;
        let asset = Asset::try_from(process.get_stack_word(0))
            .map_err(TransactionKernelError::MalformedAsset)?;

        let note_builder = self
            .output_notes
            .get_mut(&note_ptr)
            .ok_or_else(|| TransactionKernelError::MissingNote(format!("{:?}", &note_ptr)))?;

        note_builder.add_asset(asset)?;

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

    /// Extracts information from the process state about the storage map being updated and
    /// records the latest values of this storage map.
    pub fn on_account_storage_set_map_item<S: ProcessState>(
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
            TransactionEvent::AccountStorageSetMapItem => {
                self.on_account_storage_set_map_item(process)
            },
            TransactionEvent::NoteAddAsset => self.on_note_add_asset(process),
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
