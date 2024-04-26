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
            TransactionEvent::AccountStorageSetMapItem => {
                self.on_account_storage_set_map_item(process)
            },
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
        match err_code {
            131072 => ExecutionError::FailedAssertion { // 0x00020000: ERR_FAUCET_RESERVED_DATA_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("For faucets the slot FAUCET_STORAGE_DATA_SLOT is reserved and can not be used with set_account_item".to_string()),
            },
            131073 => ExecutionError::FailedAssertion { // 0x00020001: ERR_ACCT_MUST_BE_A_FAUCET
                clk: process.clk(),
                err_code,
                err_msg: Some("Procedure can only be called for faucet accounts".to_string()),
            },
            131074 => ExecutionError::FailedAssertion { // 0x00020002: ERR_P2ID_WRONG_NUMBER_OF_INPUTS
                clk: process.clk(),
                err_code,
                err_msg: Some("P2ID scripts expect exactly 1 note input".to_string()),
            },
            131075 => ExecutionError::FailedAssertion { // 0x00020003: ERR_P2ID_TARGET_ACCT_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("P2ID's target account address and transaction address do no match".to_string()),
            },
            131076 => ExecutionError::FailedAssertion { // 0x00020004: ERR_P2IDR_WRONG_NUMBER_OF_INPUTS
                clk: process.clk(),
                err_code,
                err_msg: Some("P2IDR scripts expect exactly 2 note inputs".to_string()),
            },
            131077 => ExecutionError::FailedAssertion { // 0x00020005: ERR_P2IDR_RECLAIM_ACCT_IS_NOT_SENDER
                clk: process.clk(),
                err_code,
                err_msg: Some("P2IDR's can only be reclaimed by the sender".to_string()),
            },
            131078 => ExecutionError::FailedAssertion { // 0x00020006: ERR_P2IDR_RECLAIM_HEIGHT_NOT_REACHED
                clk: process.clk(),
                err_code,
                err_msg: Some("Transaction's reference block is lower than reclaim height. The P2IDR can not be reclaimed".to_string()),
            },
            131079 => ExecutionError::FailedAssertion { // 0x00020007: ERR_SWAP_WRONG_NUMBER_OF_INPUTS
                clk: process.clk(),
                err_code,
                err_msg: Some("SWAP script expects exactly 9 note inputs".to_string()),
            },
            131080 => ExecutionError::FailedAssertion { // 0x00020008: ERR_SWAP_WRONG_NUMBER_OF_ASSETS
                clk: process.clk(),
                err_code,
                err_msg: Some("SWAP script requires exactly one note asset".to_string()),
            },
            131081 => ExecutionError::FailedAssertion { // 0x00020009: ERR_KERNEL_TX_NONCE_DID_NOT_INCREASE
                clk: process.clk(),
                err_code,
                err_msg: Some("The nonce did not increase after a state changing transaction".to_string()),
            },
            131082 => ExecutionError::FailedAssertion { // 0x0002000A: ERR_KERNEL_ASSET_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Total assets at the transaction end must match".to_string()),
            },
            131083 => ExecutionError::FailedAssertion { // 0x0002000B: ERR_PROLOGUE_GLOBAL_INPUTS_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("The global inputs provided via the advice provider do not match the block hash commitment".to_string()),
            },
            131084 => ExecutionError::FailedAssertion { // 0x0002000C: ERR_PROLOGUE_ACCT_STORAGE_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("The account storage data provided via the advice provider do not match its state commitment".to_string()),
            },
            131085 => ExecutionError::FailedAssertion { // 0x0002000D: ERR_PROLOGUE_ACCT_STORAGE_ARITY_TOO_HIGH
                clk: process.clk(),
                err_code,
                err_msg: Some("Data store in account's storage exceeds the maximum capacity of 256 elements".to_string()),
            },
            131086 => ExecutionError::FailedAssertion { // 0x0002000E: ERR_PROLOGUE_ACCT_STORAGE_TYPE_INVALID
                clk: process.clk(),
                err_code,
                err_msg: Some("Data store in account's storage contains invalid type discriminant".to_string()),
            },
            131087 => ExecutionError::FailedAssertion { // 0x0002000F: ERR_PROLOGUE_NEW_ACCT_VAULT_NOT_EMPTY
                clk: process.clk(),
                err_code,
                err_msg: Some("New account must start with an empty vault".to_string()),
            },
            131088 => ExecutionError::FailedAssertion { // 0x00020010: ERR_PROLOGUE_NEW_ACCT_INVALID_SLOT_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("New account must have valid slot type s".to_string()),
            },
            131089 => ExecutionError::FailedAssertion { // 0x00020011: ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_EMPTY_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Fungible faucet reserved slot must start empty".to_string()),
            },
            131090 => ExecutionError::FailedAssertion { // 0x00020012: ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Fungible faucet reserved slot must start with zero arity".to_string()),
            },
            131091 => ExecutionError::FailedAssertion { // 0x00020013: ERR_PROLOGUE_NEW_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Fungible faucet reserved slot must start with no type".to_string()),
            },
            131092 => ExecutionError::FailedAssertion { // 0x00020014: ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Non-fungible faucet reserved slot must start as an empty SMT".to_string()),
            },
            131093 => ExecutionError::FailedAssertion { // 0x00020015: ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_NON_ZERO_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Non-fungible faucet reserved slot must start with zero arity".to_string()),
            },
            131094 => ExecutionError::FailedAssertion { // 0x00020016: ERR_PROLOGUE_NEW_NON_FUNGIBLE_FAUCET_INVALID_TYPE_RESERVED_SLOT
                clk: process.clk(),
                err_code,
                err_msg: Some("Non-fungible faucet reserved slot must start with no type".to_string()),
            },
            131095 => ExecutionError::FailedAssertion { // 0x00020017: ERR_PROLOGUE_ACCT_HASH_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("The account data provided via advice provider did not match the initial hash".to_string()),
            },
            131096 => ExecutionError::FailedAssertion { // 0x00020018: ERR_PROLOGUE_OLD_ACCT_NONCE_ZERO
                clk: process.clk(),
                err_code,
                err_msg: Some("Existing account must not have a zero nonce".to_string()),
            },
            131097 => ExecutionError::FailedAssertion { // 0x00020019: ERR_PROLOGUE_ACCT_ID_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Account id and global account id must match".to_string()),
            },
            131098 => ExecutionError::FailedAssertion { // 0x0002001A: ERR_PROLOGUE_NOTE_MMR_DIGEST_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Reference block MMR and note's authentication MMR must match".to_string()),
            },
            131099 => ExecutionError::FailedAssertion { // 0x0002001B: ERR_PROLOGUE_NOTE_TOO_MANY_INPUTS
                clk: process.clk(),
                err_code,
                err_msg: Some("Note with too many inputs".to_string()),
            },
            131100 => ExecutionError::FailedAssertion { // 0x0002001C: ERR_PROLOGUE_NOTE_TOO_MANY_ASSETS
                clk: process.clk(),
                err_code,
                err_msg: Some("Note with too many assets".to_string()),
            },
            131101 => ExecutionError::FailedAssertion { // 0x0002001D: ERR_PROLOGUE_NOTE_CONSUMED_ASSETS_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Note's consumed assets provided via advice provider mistmatch its commitment".to_string()),
            },
            131102 => ExecutionError::FailedAssertion { // 0x0002001E: ERR_PROLOGUE_TOO_MANY_INPUT_NOTES
                clk: process.clk(),
                err_code,
                err_msg: Some("Number of input notes can no exceed the kernel's maximum limit".to_string()),
            },
            131103 => ExecutionError::FailedAssertion { // 0x0002001F: ERR_PROLOGUE_INPUT_NOTES_NULLIFIER_COMMITMENT_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Input notes nullifier commitment did not match the provided data".to_string()),
            },
            131104 => ExecutionError::FailedAssertion { // 0x00020020: ERR_TX_OUTPUT_NOTES_OVERFLOW
                clk: process.clk(),
                err_code,
                err_msg: Some("Output notes exceeded the maximum limit".to_string()),
            },
            131105 => ExecutionError::FailedAssertion { // 0x00020021: ERR_BASIC_FUNGIBLE_MAX_SUPPLY_OVERFLOW
                clk: process.clk(),
                err_code,
                err_msg: Some("Distribute would cause the max supply to be exceeded".to_string()),
            },
            131106 => ExecutionError::FailedAssertion { // 0x00020022: ERR_FAUCET_ISSUANCE_OVERFLOW
                clk: process.clk(),
                err_code,
                err_msg: Some("Asset mint operation would acuse a issuance overflow".to_string()),
            },
            131107 => ExecutionError::FailedAssertion { // 0x00020023: ERR_FAUCET_BURN_OVER_ISSUANCE
                clk: process.clk(),
                err_code,
                err_msg: Some("Asset burn can not exceed the existing supply".to_string()),
            },
            131108 => ExecutionError::FailedAssertion { // 0x00020024: ERR_FAUCET_NON_FUNGIBLE_ALREADY_EXISTS
                clk: process.clk(),
                err_code,
                err_msg: Some("Non fungible token already exists, it can be issue only once".to_string()),
            },
            131109 => ExecutionError::FailedAssertion { // 0x00020025: ERR_FAUCET_NON_FUNGIBLE_BURN_WRONG_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("Non fungible burn called on the wrong faucet type".to_string()),
            },
            131110 => ExecutionError::FailedAssertion { // 0x00020026: ERR_FAUCET_NONEXISTING_TOKEN
                clk: process.clk(),
                err_code,
                err_msg: Some("Non fungible burn called on inexisting token".to_string()),
            },
            131111 => ExecutionError::FailedAssertion { // 0x00020027: ERR_NOTE_INVALID_SENDER
                clk: process.clk(),
                err_code,
                err_msg: Some("Input note can not have an empty sender, procedure was likely called from the wrong context".to_string()),
            },
            131112 => ExecutionError::FailedAssertion { // 0x00020028: ERR_NOTE_INVALID_VAULT
                clk: process.clk(),
                err_code,
                err_msg: Some("Input note can not have an empty vault, procedure was likely called from the wrong context".to_string()),
            },
            131113 => ExecutionError::FailedAssertion { // 0x00020029: ERR_NOTE_INVALID_INPUTS
                clk: process.clk(),
                err_code,
                err_msg: Some("Input note can not have empty inputs, procedure was likely called from the wrong context".to_string()),
            },
            131114 => ExecutionError::FailedAssertion { // 0x0002002A: ERR_NOTE_TOO_MANY_ASSETS
                clk: process.clk(),
                err_code,
                err_msg: Some("Note's asset must fit in a u32".to_string()),
            },
            131115 => ExecutionError::FailedAssertion { // 0x0002002B: ERR_VAULT_GET_BALANCE_WRONG_ASSET_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("The get_balance procedure can be called only with a fungible faucet".to_string()),
            },
            131116 => ExecutionError::FailedAssertion { // 0x0002002C: ERR_VAULT_HAS_NON_FUNGIBLE_WRONG_ACCOUNT_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("The has_non_fungible_asset procedure can be called only with a non-fungible faucet".to_string()),
            },
            131117 => ExecutionError::FailedAssertion { // 0x0002002D: ERR_VAULT_FUNGIBLE_MAX_AMOUNT_EXCEEDED
                clk: process.clk(),
                err_code,
                err_msg: Some("Adding the fungible asset would exceed the max_amount".to_string()),
            },
            131118 => ExecutionError::FailedAssertion { // 0x0002002E: ERR_VAULT_ADD_FUNGIBLE_ASSET_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Decorator value did not match the assert commitment".to_string()),
            },
            131119 => ExecutionError::FailedAssertion { // 0x0002002F: ERR_VAULT_NON_FUNGIBLE_ALREADY_EXISTED
                clk: process.clk(),
                err_code,
                err_msg: Some("The non-fungible asset already existed, can not be added again".to_string()),
            },
            131120 => ExecutionError::FailedAssertion { // 0x00020030: ERR_VAULT_FUNGIBLE_AMOUNT_UNDERFLOW
                clk: process.clk(),
                err_code,
                err_msg: Some("Removing the fungible asset would have current amount being negative".to_string()),
            },
            131121 => ExecutionError::FailedAssertion { // 0x00020031: ERR_VAULT_REMOVE_FUNGIBLE_ASSET_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Data provided via decorator did not match the commitment".to_string()),
            },
            131122 => ExecutionError::FailedAssertion { // 0x00020032: ERR_VAULT_NON_FUNGIBLE_MISSING_ASSET
                clk: process.clk(),
                err_code,
                err_msg: Some("Removing inexisting non-fungible asset".to_string()),
            },
            131123 => ExecutionError::FailedAssertion { // 0x00020033: ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_BE_ZERO
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 1 must be zero".to_string()),
            },
            131124 => ExecutionError::FailedAssertion { // 0x00020034: ERR_ASSET_FORMAT_POSITION_TWO_MUST_BE_ZERO
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 2 must be zero".to_string()),
            },
            131125 => ExecutionError::FailedAssertion { // 0x00020035: ERR_FUNGIBLE_ASSET_FORMAT_POSITION_THREE_MUST_BE_ZERO
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 3 must correspond to a fungible".to_string()),
            },
            131126 => ExecutionError::FailedAssertion { // 0x00020036: ERR_FUNGIBLE_ASSET_FORMAT_POSITION_ZERO_MUST_BE_ZERO
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 0 must be within limit".to_string()),
            },
            131127 => ExecutionError::FailedAssertion { // 0x00020037: ERR_NON_FUNGIBLE_ASSET_FORMAT_POSITION_ONE_MUST_FUNGIBLE
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 1 must be zero".to_string()),
            },
            131128 => ExecutionError::FailedAssertion { // 0x00020038: ERR_NON_FUNGIBLE_ASSET_HIGH_BIT_SET
                clk: process.clk(),
                err_code,
                err_msg: Some("The felt at position 3 must be zero".to_string()),
            },
            131129 => ExecutionError::FailedAssertion { // 0x00020039: ERR_FUNGIBLE_ASSET_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Fungible asset origin validation failed".to_string()),
            },
            131130 => ExecutionError::FailedAssertion { // 0x0002003A: ERR_NON_FUNGIBLE_ASSET_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Non-fungible asset origin validation failed".to_string()),
            },
            131131 => ExecutionError::FailedAssertion { // 0x0002003B: ERR_ACCOUNT_NONCE_INCR_MUST_BE_U32
                clk: process.clk(),
                err_code,
                err_msg: Some("The nonce increase must be a u32".to_string()),
            },
            131132 => ExecutionError::FailedAssertion { // 0x0002003C: ERR_ACCOUNT_INSUFFICIENT_ONES
                clk: process.clk(),
                err_code,
                err_msg: Some("Account id format is invalid, insufficient ones".to_string()),
            },
            131133 => ExecutionError::FailedAssertion { // 0x0002003D: ERR_ACCOUNT_SET_CODE_ACCOUNT_MUST_BE_UPDATABLE
                clk: process.clk(),
                err_code,
                err_msg: Some("Account must be updatable for it to be possible to update its code".to_string()),
            },
            131134 => ExecutionError::FailedAssertion { // 0x0002003E: ERR_ACCOUNT_SEED_DIGEST_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Account seed digest mismatch".to_string()),
            },
            131135 => ExecutionError::FailedAssertion { // 0x0002003F: ERR_ACCOUNT_INVALID_POW
                clk: process.clk(),
                err_code,
                err_msg: Some("Account pow is insufficient".to_string()),
            },
            131136 => ExecutionError::FailedAssertion { // 0x00020040: ERR_NOTE_DATA_MISMATCH
                clk: process.clk(),
                err_code,
                err_msg: Some("Note's advice data does not match the expected commitment".to_string()),
            },
            131137 => ExecutionError::FailedAssertion { // 0x00020041: ERR_ASSET_NOT_FUNGIBLE_ID
                clk: process.clk(),
                err_code,
                err_msg: Some("Can not build the fungible asset because provided id is not a fungible id".to_string()),
            },
            131138 => ExecutionError::FailedAssertion { // 0x00020042: ERR_ASSET_INVALID_AMOUNT
                clk: process.clk(),
                err_code,
                err_msg: Some("Can not build the asset because amount exceeds the maximum".to_string()),
            },
            131139 => ExecutionError::FailedAssertion { // 0x00020043: ERR_ASSET_NOT_NON_FUNGIBLE_ID
                clk: process.clk(),
                err_code,
                err_msg: Some("Can not build the non-fungible asset because provided id is not a non-fungible id".to_string()),
            },
            131140 => ExecutionError::FailedAssertion { // 0x00020044: ERR_INVALID_NOTE_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("Invalid note type".to_string()),
            },
            131141 => ExecutionError::FailedAssertion { // 0x00020045: ERR_NOTE_INVALID_TAG_PREFIX_FOR_TYPE
                clk: process.clk(),
                err_code,
                err_msg: Some("The note's tag failed the most significant validation".to_string()),
            },
            131142 => ExecutionError::FailedAssertion { // 0x00020046: ERR_NOTE_INVALID_TAG_HIGH_BIT_SET
                clk: process.clk(),
                err_code,
                err_msg: Some("The note's tag high bits must be set to zero".to_string()),
            },
            _ => ExecutionError::FailedAssertion {
                clk: process.clk(),
                err_code,
                err_msg: Some("Unknown error code".to_string()),
            }
        }
    }
}
