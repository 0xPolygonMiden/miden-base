use core::cell::OnceCell;

use super::{
    accounts::AccountId,
    assembly::{Assembler, AssemblyContext, ProgramAst},
    assets::Asset,
    utils::{
        collections::Vec,
        serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
        string::String,
    },
    vm::CodeBlock,
    Digest, Felt, Hasher, NoteError, Word, NOTE_TREE_DEPTH, WORD_SIZE, ZERO,
};

mod envelope;
pub use envelope::NoteEnvelope;

mod inputs;
pub use inputs::NoteInputs;

mod metadata;
pub use metadata::NoteMetadata;

mod note_id;
pub use note_id::NoteId;

mod nullifier;
pub use nullifier::Nullifier;

mod location;
pub use location::NoteLocation;

mod script;
pub use script::NoteScript;

mod assets;
pub use assets::NoteAssets;

// CONSTANTS
// ================================================================================================

/// The depth of the leafs in the note Merkle tree used to commit to notes produced in a block.
/// This is equal `NOTE_TREE_DEPTH + 1`. In the kernel we do not authenticate leaf data directly
/// but rather authenticate hash(left_leaf, right_leaf).
pub const NOTE_LEAF_DEPTH: u8 = NOTE_TREE_DEPTH + 1;

// NOTE
// ================================================================================================

/// A note which can be used to transfer assets between accounts.
///
/// This struct is a full description of a note which is needed to execute a note in a transaction.
/// A note consists of:
///
/// Core on-chain data which is used to execute a note:
/// - A script which must be executed in a context of some account to claim the assets.
/// - A set of inputs which can be read to memory during script execution via the invocation of the
///   `note::get_inputs` in the kernel API.
/// - A set of assets stored in the note.
/// - A serial number which can be used to break linkability between note hash and note nullifier.
///
/// Auxiliary data which is used to verify authenticity and signal additional information:
/// - A metadata object which contains information about the sender, the tag and the number of
///   assets in the note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    script: NoteScript,
    inputs: NoteInputs,
    assets: NoteAssets,
    serial_num: Word,
    metadata: NoteMetadata,

    id: OnceCell<NoteId>,
    nullifier: OnceCell<Nullifier>,
}

impl Note {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new note created with the specified parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of inputs exceeds 16.
    /// - The number of provided assets exceeds 1000.
    /// - The list of assets contains duplicates.
    pub fn new(
        script: NoteScript,
        inputs: &[Felt],
        assets: &[Asset],
        serial_num: Word,
        sender: AccountId,
        tag: Felt,
    ) -> Result<Self, NoteError> {
        Ok(Self {
            script,
            inputs: NoteInputs::new(inputs.to_vec())?,
            assets: NoteAssets::new(assets)?,
            serial_num,
            metadata: NoteMetadata::new(sender, tag),
            id: OnceCell::new(),
            nullifier: OnceCell::new(),
        })
    }

    /// Returns a note instance created from the provided parts.
    pub fn from_parts(
        script: NoteScript,
        inputs: NoteInputs,
        assets: NoteAssets,
        serial_num: Word,
        metadata: NoteMetadata,
    ) -> Self {
        Self {
            script,
            inputs,
            assets,
            serial_num,
            metadata,
            id: OnceCell::new(),
            nullifier: OnceCell::new(),
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference script which locks the assets of this note.
    pub fn script(&self) -> &NoteScript {
        &self.script
    }

    /// Returns a reference to the note inputs.
    pub fn inputs(&self) -> &NoteInputs {
        &self.inputs
    }

    /// Returns a reference to the asset of this note.
    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    /// Returns a serial number of this note.
    pub fn serial_num(&self) -> Word {
        self.serial_num
    }

    /// Returns the metadata associated with this note.
    pub fn metadata(&self) -> &NoteMetadata {
        &self.metadata
    }

    /// Returns the recipient of this note.
    /// Recipient is defined and calculated as:
    ///  hash(hash(hash(serial_num, [0; 4]), script_hash), input_hash)
    pub fn recipient(&self) -> Digest {
        let serial_num_hash = Hasher::merge(&[self.serial_num.into(), Digest::default()]);
        let merge_script = Hasher::merge(&[serial_num_hash, self.script.hash()]);
        Hasher::merge(&[merge_script, self.inputs.commitment()])
    }

    /// Returns a unique identifier of this note, which is simultaneously a commitment to the note.
    pub fn id(&self) -> NoteId {
        *self.id.get_or_init(|| self.into())
    }

    /// Returns the value used to authenticate a notes existence in the note tree. This is computed
    /// as a 2-to-1 hash of the note hash and note metadata [hash(note_id, note_metadata)]
    pub fn authentication_hash(&self) -> Digest {
        Hasher::merge(&[self.id().inner(), Word::from(self.metadata()).into()])
    }

    /// Returns the nullifier for this note.
    pub fn nullifier(&self) -> Nullifier {
        *self.nullifier.get_or_init(|| self.into())
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Note {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let Note {
            script,
            inputs,
            assets,
            serial_num,
            metadata,

            id: _,
            nullifier: _,
        } = self;

        script.write_into(target);
        inputs.write_into(target);
        assets.write_into(target);
        serial_num.write_into(target);
        metadata.write_into(target);
    }
}

impl Deserializable for Note {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let script = NoteScript::read_from(source)?;
        let inputs = NoteInputs::read_from(source)?;
        let assets = NoteAssets::read_from(source)?;
        let serial_num = Word::read_from(source)?;
        let metadata = NoteMetadata::read_from(source)?;

        Ok(Self {
            script,
            inputs,
            assets,
            serial_num,
            metadata,
            id: OnceCell::new(),
            nullifier: OnceCell::new(),
        })
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Note {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = self.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Note {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;
        Self::read_from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}
