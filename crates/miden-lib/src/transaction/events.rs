use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};
use core::fmt;

use miden_objects::asset::Asset;
use vm_processor::{AdviceProvider, AdviceSource, EventHandler, ProcessState};

use super::{
    memory::NATIVE_NUM_ACCT_STORAGE_SLOTS_PTR, AccountProcedureIndexMap, OutputNoteBuilder,
    TransactionEventError, TransactionKernelError, TransactionTraceParsingError,
};
use crate::{account_delta_tracker::AccountDeltaTracker, utils::sync::RwLock};

// CONSTANTS
// ================================================================================================

// TRANSACTION EVENT
// ================================================================================================

const ACCOUNT_VAULT_BEFORE_ADD_ASSET: u32 = 0x2_0000; // 131072
const ACCOUNT_VAULT_AFTER_ADD_ASSET: u32 = 0x2_0001; // 131073

const ACCOUNT_VAULT_BEFORE_REMOVE_ASSET: u32 = 0x2_0002; // 131074
const ACCOUNT_VAULT_AFTER_REMOVE_ASSET: u32 = 0x2_0003; // 131075

const ACCOUNT_STORAGE_BEFORE_SET_ITEM: u32 = 0x2_0004; // 131076
const ACCOUNT_STORAGE_AFTER_SET_ITEM: u32 = 0x2_0005; // 131077

const ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM: u32 = 0x2_0006; // 131078
const ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM: u32 = 0x2_0007; // 131079

const ACCOUNT_BEFORE_INCREMENT_NONCE: u32 = 0x2_0008; // 131080
const ACCOUNT_AFTER_INCREMENT_NONCE: u32 = 0x2_0009; // 131081

const ACCOUNT_PUSH_PROCEDURE_INDEX: u32 = 0x2_000a; // 131082

const NOTE_BEFORE_CREATED: u32 = 0x2_000b; // 131083
const NOTE_AFTER_CREATED: u32 = 0x2_000c; // 131084

const NOTE_BEFORE_ADD_ASSET: u32 = 0x2_000d; // 131085
const NOTE_AFTER_ADD_ASSET: u32 = 0x2_000e; // 131086

/// Events which may be emitted by a transaction kernel.
///
/// The events are emitted via the `emit.<event_id>` instruction. The event ID is a 32-bit
/// unsigned integer which is used to identify the event type. For events emitted by the
/// transaction kernel, the event_id is structured as follows:
/// - The upper 16 bits of the event ID are set to 2.
/// - The lower 16 bits represent a unique event ID within the transaction kernel.
#[repr(u32)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionEvent {
    AccountVaultBeforeAddAsset = ACCOUNT_VAULT_BEFORE_ADD_ASSET,
    AccountVaultAfterAddAsset = ACCOUNT_VAULT_AFTER_ADD_ASSET,

    AccountVaultBeforeRemoveAsset = ACCOUNT_VAULT_BEFORE_REMOVE_ASSET,
    AccountVaultAfterRemoveAsset = ACCOUNT_VAULT_AFTER_REMOVE_ASSET,

    AccountStorageBeforeSetItem = ACCOUNT_STORAGE_BEFORE_SET_ITEM,
    AccountStorageAfterSetItem = ACCOUNT_STORAGE_AFTER_SET_ITEM,

    AccountStorageBeforeSetMapItem = ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM,
    AccountStorageAfterSetMapItem = ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM,

    AccountBeforeIncrementNonce = ACCOUNT_BEFORE_INCREMENT_NONCE,
    AccountAfterIncrementNonce = ACCOUNT_AFTER_INCREMENT_NONCE,

    AccountPushProcedureIndex = ACCOUNT_PUSH_PROCEDURE_INDEX,

    NoteBeforeCreated = NOTE_BEFORE_CREATED,
    NoteAfterCreated = NOTE_AFTER_CREATED,

    NoteBeforeAddAsset = NOTE_BEFORE_ADD_ASSET,
    NoteAfterAddAsset = NOTE_AFTER_ADD_ASSET,
}

impl TransactionEvent {
    /// Value of the top 16 bits of a transaction kernel event ID.
    pub const ID_PREFIX: u32 = 2;
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionEvent {
    type Error = TransactionEventError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != TransactionEvent::ID_PREFIX {
            return Err(TransactionEventError::NotTransactionEvent(value));
        }

        match value {
            ACCOUNT_VAULT_BEFORE_ADD_ASSET => Ok(TransactionEvent::AccountVaultBeforeAddAsset),
            ACCOUNT_VAULT_AFTER_ADD_ASSET => Ok(TransactionEvent::AccountVaultAfterAddAsset),

            ACCOUNT_VAULT_BEFORE_REMOVE_ASSET => {
                Ok(TransactionEvent::AccountVaultBeforeRemoveAsset)
            },
            ACCOUNT_VAULT_AFTER_REMOVE_ASSET => Ok(TransactionEvent::AccountVaultAfterRemoveAsset),

            ACCOUNT_STORAGE_BEFORE_SET_ITEM => Ok(TransactionEvent::AccountStorageBeforeSetItem),
            ACCOUNT_STORAGE_AFTER_SET_ITEM => Ok(TransactionEvent::AccountStorageAfterSetItem),

            ACCOUNT_STORAGE_BEFORE_SET_MAP_ITEM => {
                Ok(TransactionEvent::AccountStorageBeforeSetMapItem)
            },
            ACCOUNT_STORAGE_AFTER_SET_MAP_ITEM => {
                Ok(TransactionEvent::AccountStorageAfterSetMapItem)
            },

            ACCOUNT_BEFORE_INCREMENT_NONCE => Ok(TransactionEvent::AccountBeforeIncrementNonce),
            ACCOUNT_AFTER_INCREMENT_NONCE => Ok(TransactionEvent::AccountAfterIncrementNonce),

            ACCOUNT_PUSH_PROCEDURE_INDEX => Ok(TransactionEvent::AccountPushProcedureIndex),

            NOTE_BEFORE_CREATED => Ok(TransactionEvent::NoteBeforeCreated),
            NOTE_AFTER_CREATED => Ok(TransactionEvent::NoteAfterCreated),

            NOTE_BEFORE_ADD_ASSET => Ok(TransactionEvent::NoteBeforeAddAsset),
            NOTE_AFTER_ADD_ASSET => Ok(TransactionEvent::NoteAfterAddAsset),

            _ => Err(TransactionEventError::InvalidTransactionEvent(value)),
        }
    }
}

// TRANSACTION TRACE
// ================================================================================================

#[repr(u32)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransactionTrace {
    PrologueStart = 0x2_0000,           // 131072
    PrologueEnd = 0x2_0001,             // 131073
    NotesProcessingStart = 0x2_0002,    // 131074
    NotesProcessingEnd = 0x2_0003,      // 131075
    NoteExecutionStart = 0x2_0004,      // 131076
    NoteExecutionEnd = 0x2_0005,        // 131077
    TxScriptProcessingStart = 0x2_0006, // 131078
    TxScriptProcessingEnd = 0x2_0007,   // 131079
    EpilogueStart = 0x2_0008,           // 131080
    EpilogueEnd = 0x2_0009,             // 131081
}

impl fmt::Display for TransactionTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TryFrom<u32> for TransactionTrace {
    type Error = TransactionTraceParsingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >> 16 != TransactionEvent::ID_PREFIX {
            return Err(TransactionTraceParsingError::UnknownTransactionTrace(value));
        }

        match value {
            0x2_0000 => Ok(TransactionTrace::PrologueStart),
            0x2_0001 => Ok(TransactionTrace::PrologueEnd),
            0x2_0002 => Ok(TransactionTrace::NotesProcessingStart),
            0x2_0003 => Ok(TransactionTrace::NotesProcessingEnd),
            0x2_0004 => Ok(TransactionTrace::NoteExecutionStart),
            0x2_0005 => Ok(TransactionTrace::NoteExecutionEnd),
            0x2_0006 => Ok(TransactionTrace::TxScriptProcessingStart),
            0x2_0007 => Ok(TransactionTrace::TxScriptProcessingEnd),
            0x2_0008 => Ok(TransactionTrace::EpilogueStart),
            0x2_0009 => Ok(TransactionTrace::EpilogueEnd),
            _ => Err(TransactionTraceParsingError::UnknownTransactionTrace(value)),
        }
    }
}

// EVENT HANDLERS
// ================================================================================================

#[derive(Debug, Clone)]
pub struct AccountVaultAfterAddAssetHandler {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
}

impl<A> EventHandler<A> for AccountVaultAfterAddAssetHandler {
    fn id(&self) -> u32 {
        TransactionEvent::AccountVaultAfterAddAsset as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let asset: Asset = process.get_stack_word(0).try_into().map_err(|source| {
            TransactionKernelError::MalformedAssetInEventHandler {
                handler: "on_account_vault_after_add_asset",
                source,
            }
        })?;

        self.account_delta
            .write()
            .vault_delta()
            .add_asset(asset)
            .map_err(TransactionKernelError::AccountDeltaAddAssetFailed)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AccountVaultAfterRemoveAssetHandler {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
}

impl<A> EventHandler<A> for AccountVaultAfterRemoveAssetHandler {
    fn id(&self) -> u32 {
        TransactionEvent::AccountVaultAfterRemoveAsset as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let asset: Asset = process.get_stack_word(0).try_into().map_err(|source| {
            TransactionKernelError::MalformedAssetInEventHandler {
                handler: "on_account_vault_after_remove_asset",
                source,
            }
        })?;

        self.account_delta
            .write()
            .vault_delta()
            .remove_asset(asset)
            .map_err(TransactionKernelError::AccountDeltaRemoveAssetFailed)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AccountStorageAfterSetItemHandler {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
}

impl<A> EventHandler<A> for AccountStorageAfterSetItemHandler {
    fn id(&self) -> u32 {
        TransactionEvent::AccountStorageAfterSetItem as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        // get slot index from the stack and make sure it is valid
        let slot_index = process.get_stack_item(0);

        // get number of storage slots initialized by the account
        let num_storage_slot = get_num_storage_slots(process)?;

        if slot_index.as_int() >= num_storage_slot {
            return Err(TransactionKernelError::InvalidStorageSlotIndex {
                max: num_storage_slot,
                actual: slot_index.as_int(),
            }
            .into());
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
            self.account_delta.write().storage_delta().set_item(slot_index, new_slot_value);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AccountStorageAfterSetMapItemHandler {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
}

impl<A> EventHandler<A> for AccountStorageAfterSetMapItemHandler {
    fn id(&self) -> u32 {
        TransactionEvent::AccountStorageAfterSetMapItem as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        // get slot index from the stack and make sure it is valid
        let slot_index = process.get_stack_item(0);

        // get number of storage slots initialized by the account
        let num_storage_slot = get_num_storage_slots(process)?;

        if slot_index.as_int() >= num_storage_slot {
            return Err(TransactionKernelError::InvalidStorageSlotIndex {
                max: num_storage_slot,
                actual: slot_index.as_int(),
            }
            .into());
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
        self.account_delta.write().storage_delta().set_map_item(
            slot_index,
            new_map_key.into(),
            new_map_value,
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AccountBeforeIncrementNonceHandler {
    /// Account state changes accumulated during transaction execution.
    pub account_delta: Arc<RwLock<AccountDeltaTracker>>,
}

impl<A> EventHandler<A> for AccountBeforeIncrementNonceHandler {
    fn id(&self) -> u32 {
        TransactionEvent::AccountBeforeIncrementNonce as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let value = process.get_stack_item(0);
        self.account_delta.write().increment_nonce(value);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AccountPushProcedureIndexHandler {
    /// A map of the account's procedure MAST roots to the corresponding procedure indexes in the
    /// account code.
    pub acct_procedure_index_map: AccountProcedureIndexMap,
}

impl<A> EventHandler<A> for AccountPushProcedureIndexHandler
where
    A: AdviceProvider,
{
    fn id(&self) -> u32 {
        TransactionEvent::AccountPushProcedureIndex as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let proc_idx = self.acct_procedure_index_map.get_proc_index(&process)?;
        advice_provider
            .push_stack(AdviceSource::Value(proc_idx.into()))
            .expect("failed to push value onto advice stack");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NoteAfterCreatedHandler {
    /// The list of notes created while executing a transaction stored as note_ptr |-> note_builder
    /// map.
    pub output_notes: Arc<RwLock<BTreeMap<usize, OutputNoteBuilder>>>,
}

impl<A> EventHandler<A> for NoteAfterCreatedHandler
where
    A: AdviceProvider,
{
    fn id(&self) -> u32 {
        TransactionEvent::NoteAfterCreated as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let stack = process.get_stack_state();
        // # => [NOTE_METADATA]

        let note_idx: usize = stack[9].as_int() as usize;

        let mut output_notes = self.output_notes.write();
        assert_eq!(note_idx, output_notes.len(), "note index mismatch");

        let note_builder = OutputNoteBuilder::new(stack, advice_provider)?;

        output_notes.insert(note_idx, note_builder);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NoteBeforeAddAssetHandler {
    /// The list of notes created while executing a transaction stored as note_ptr |-> note_builder
    /// map.
    pub output_notes: Arc<RwLock<BTreeMap<usize, OutputNoteBuilder>>>,
}

impl<A> EventHandler<A> for NoteBeforeAddAssetHandler
where
    A: AdviceProvider,
{
    fn id(&self) -> u32 {
        TransactionEvent::NoteBeforeAddAsset as u32
    }

    fn on_event(
        &mut self,
        process: ProcessState,
        _advice_provider: &mut A,
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        let stack = process.get_stack_state();
        //# => [ASSET, note_ptr, num_of_assets, note_idx]

        let note_idx = stack[6].as_int();
        let mut output_notes = self.output_notes.write();

        assert!(note_idx < output_notes.len() as u64);
        let node_idx = note_idx as usize;

        let asset = Asset::try_from(process.get_stack_word(0)).map_err(|source| {
            TransactionKernelError::MalformedAssetInEventHandler {
                handler: "on_note_before_add_asset",
                source,
            }
        })?;

        let note_builder = output_notes
            .get_mut(&node_idx)
            .ok_or_else(|| TransactionKernelError::MissingNote(note_idx))?;

        note_builder.add_asset(asset)?;

        Ok(())
    }
}

// HELPERS
// ================================================================================================

/// Returns the number of storage slots initialized for the current account.
///
/// # Errors
/// Returns an error if the memory location supposed to contain the account storage slot number
/// has not been initialized.
fn get_num_storage_slots(process: ProcessState) -> Result<u64, TransactionKernelError> {
    let num_storage_slots_felt = process
        .get_mem_value(process.ctx(), NATIVE_NUM_ACCT_STORAGE_SLOTS_PTR)
        .ok_or(TransactionKernelError::AccountStorageSlotsNumMissing(
            NATIVE_NUM_ACCT_STORAGE_SLOTS_PTR,
        ))?;

    Ok(num_storage_slots_felt.as_int())
}
