// NOTE EXECUTION HINT
// ================================================================================================

use vm_core::Felt;

use crate::NoteError;

// CONSTANTS
// ================================================================================================

const NONE_TAG: u8 = 0;
const ALWAYS_TAG: u8 = 1;
const AFTER_BLOCK_TAG: u8 = 2;
const ON_BLOCK_SLOT_TAG: u8 = 3;

/// Specifies the conditions under which a note is ready to be consumed.
/// These conditions are meant to be encoded in the note script as well.
///
/// This struct can be represented as the combination of a tag, and a payload.
/// The tag specifies the variant of the hint, and the payload encodes the hint data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NoteExecutionHint {
    /// Unspecified note execution hint. Implies it is not knorn under which conditions the note
    /// is consumable.
    None,
    /// The note's script can be executed at any time.
    Always,
    /// The note's script can be executed after the specified block height.
    AfterBlock { block_num: u32 },
    /// The note's script can be executed in the specified slot within the specified epoch.
    ///
    /// The slot is defined as follows:
    /// - First we define the length of the epoch in powers of 2. For example, epoch_len = 10 is
    ///   an epoch of 1024 blocks.
    /// - Then we define the length of a slot within the epoch also using powers of 2. For example,
    ///   slot_len = 7 is a slot of 128 blocks.
    /// - Lastly, the offset specifies the index of the slot within the epoch - i.e., 0 is the
    ///   first slot, 1 is the second slot etc.
    ///
    /// For example: { epoch_len: 10, slot_len: 7, slot_offset: 1 } means that the note can
    /// be executed in any second 128 block slot of a 1024 block epoch. These would be blocks 128..255,
    /// 1152..1279, 2176..2303 etc.
    OnBlockSlot {
        epoch_len: u8,
        slot_len: u8,
        slot_offset: u8,
    },
}

impl NoteExecutionHint {
    // CONSTRUCTORS
    // ------------------------------------------------------------------------------------------------

    /// Creates a [NoteExecutionHint::None] variant
    pub fn none() -> Self {
        NoteExecutionHint::None
    }

    /// Creates a [NoteExecutionHint::Always] variant
    pub fn always() -> Self {
        NoteExecutionHint::Always
    }

    /// Creates a [NoteExecutionHint::AfterBlock] variant based on the given `block_num`
    pub fn after_block(block_num: u32) -> Self {
        NoteExecutionHint::AfterBlock { block_num }
    }

    /// Creates a [NoteExecutionHint::OnBlockSlot] for the given parameters
    pub fn on_block_slot(epoch_len: u8, slot_len: u8, slot_offset: u8) -> Self {
        NoteExecutionHint::OnBlockSlot { epoch_len, slot_len, slot_offset }
    }

    pub fn from_parts(tag: u8, payload: u32) -> Result<NoteExecutionHint, NoteError> {
        match tag {
            NONE_TAG => {
                if payload != 0 {
                    return Err(NoteError::InvalidNoteExecutionHintPayload(tag, payload));
                }
                Ok(NoteExecutionHint::None)
            },
            ALWAYS_TAG => {
                if payload != 0 {
                    return Err(NoteError::InvalidNoteExecutionHintPayload(tag, payload));
                }
                Ok(NoteExecutionHint::Always)
            },
            AFTER_BLOCK_TAG => Ok(NoteExecutionHint::AfterBlock { block_num: payload }),
            ON_BLOCK_SLOT_TAG => {
                let remainder = (payload >> 24 & 0xFF) as u8;
                if remainder != 0 {
                    return Err(NoteError::InvalidNoteExecutionHintPayload(tag, payload));
                }

                let epoch_len = ((payload >> 16) & 0xFF) as u8;
                let slot_len = ((payload >> 8) & 0xFF) as u8;
                let slot_offset = (payload & 0xFF) as u8;
                let hint = NoteExecutionHint::OnBlockSlot { epoch_len, slot_len, slot_offset };

                Ok(hint)
            },
            _ => Err(NoteError::InvalidNoteExecutionHintTag(tag)),
        }
    }

    /// Returns whether the note execution conditions validate for the given `block_num`
    ///
    /// # Returns
    /// - `None` if we don't know whether the note can be consumed.
    /// - `Some(true)` if the note is consumable for the given `block_num`
    /// - `Some(false)` if the note is not consumable for the given `block_num`
    pub fn can_be_consumed(&self, block_num: u32) -> Option<bool> {
        match self {
            NoteExecutionHint::None => None,
            NoteExecutionHint::Always => Some(true),
            NoteExecutionHint::AfterBlock { block_num: hint_block_num } => {
                Some(block_num >= *hint_block_num)
            },
            NoteExecutionHint::OnBlockSlot { epoch_len, slot_len, slot_offset } => {
                let epoch_len_blocks: u32 = 1 << epoch_len;
                let slot_len_blocks: u32 = 1 << slot_len;

                let block_epoch_index = block_num / epoch_len_blocks;

                let slot_start_block =
                    block_epoch_index * epoch_len_blocks + (*slot_offset as u32) * slot_len_blocks;
                let slot_end_block = slot_start_block + slot_len_blocks;

                let can_be_consumed = block_num >= slot_start_block && block_num < slot_end_block;
                Some(can_be_consumed)
            },
        }
    }

    pub fn into_parts(&self) -> (u8, u32) {
        match self {
            NoteExecutionHint::None => (NONE_TAG, 0),
            NoteExecutionHint::Always => (ALWAYS_TAG, 0),
            NoteExecutionHint::AfterBlock { block_num } => (AFTER_BLOCK_TAG, *block_num),
            NoteExecutionHint::OnBlockSlot { epoch_len, slot_len, slot_offset } => {
                let payload: u32 =
                    ((*epoch_len as u32) << 16) | ((*slot_len as u32) << 8) | (*slot_offset as u32);
                (ON_BLOCK_SLOT_TAG, payload)
            },
        }
    }
}

/// As a Felt, the ExecutionHint is encoded as:
///
/// - 6 least significant bits: Hint identifier (tag).
/// - Bits 6 to 38: Hint payload.
///
/// This way, hints such as [NoteExecutionHint::Always], are represented by `Felt::new(1)`
impl From<NoteExecutionHint> for Felt {
    fn from(value: NoteExecutionHint) -> Self {
        let (tag, payload) = value.into_parts();
        Felt::new(((payload as u64) << 6) | (tag as u64))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_hint_serde(note_execution_hint: NoteExecutionHint) {
        let (tag, payload) = note_execution_hint.into_parts();
        let deserialized = NoteExecutionHint::from_parts(tag, payload).unwrap();
        assert_eq!(deserialized, note_execution_hint);
    }

    #[test]
    fn test_serialization_round_trip() {
        assert_hint_serde(NoteExecutionHint::None);
        assert_hint_serde(NoteExecutionHint::Always);
        assert_hint_serde(NoteExecutionHint::AfterBlock { block_num: 15 });
        assert_hint_serde(NoteExecutionHint::OnBlockSlot {
            epoch_len: 9,
            slot_len: 12,
            slot_offset: 18,
        });
    }

    #[test]
    fn test_can_be_consumed() {
        let none = NoteExecutionHint::none();
        assert!(none.can_be_consumed(100).is_none());

        let always = NoteExecutionHint::always();
        assert!(always.can_be_consumed(100).unwrap());

        let after_block = NoteExecutionHint::after_block(12345);
        assert!(!after_block.can_be_consumed(12344).unwrap());
        assert!(after_block.can_be_consumed(12345).unwrap());

        let on_block_slot = NoteExecutionHint::on_block_slot(10, 7, 1);
        assert!(!on_block_slot.can_be_consumed(127).unwrap()); // Block 127 is not in the slot 128..255
        assert!(on_block_slot.can_be_consumed(128).unwrap()); // Block 128 is in the slot 128..255
        assert!(on_block_slot.can_be_consumed(255).unwrap()); // Block 255 is in the slot 128..255
        assert!(!on_block_slot.can_be_consumed(256).unwrap()); // Block 256 is not in the slot 128..255
        assert!(on_block_slot.can_be_consumed(1152).unwrap()); // Block 1152 is in the slot 1152..1279
        assert!(on_block_slot.can_be_consumed(1279).unwrap()); // Block 1279 is in the slot 1152..1279
        assert!(on_block_slot.can_be_consumed(2176).unwrap()); // Block 2176 is in the slot 2176..2303
        assert!(!on_block_slot.can_be_consumed(2175).unwrap()); // Block 2175 is not in the slot 2176..2303
    }

    #[test]
    fn test_parts_validity() {
        NoteExecutionHint::from_parts(NONE_TAG, 1).unwrap_err();
        NoteExecutionHint::from_parts(ALWAYS_TAG, 12).unwrap_err();
        // 4th byte should be blank for tag 3 (OnBlockSlot)
        NoteExecutionHint::from_parts(ON_BLOCK_SLOT_TAG, 1 << 24).unwrap_err();
        NoteExecutionHint::from_parts(ON_BLOCK_SLOT_TAG, 0).unwrap();

        NoteExecutionHint::from_parts(10, 1).unwrap_err();
    }
}
