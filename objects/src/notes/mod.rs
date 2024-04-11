use miden_crypto::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Word,
};
use vm_processor::DeserializationError;

use crate::{
    accounts::AccountId,
    assembly::{Assembler, AssemblyContext, ProgramAst},
    assets::Asset,
    vm::CodeBlock,
    Digest, Felt, Hasher, NoteError, NOTE_TREE_DEPTH, WORD_SIZE, ZERO,
};

mod assets;
pub use assets::NoteAssets;

mod envelope;
pub use envelope::NoteEnvelope;

mod inputs;
pub use inputs::NoteInputs;

mod metadata;
pub use metadata::{NoteExecutionMode, NoteMetadata};

mod note_id;
pub use note_id::NoteId;

mod note_tag;
pub use note_tag::NoteTag;

mod note_type;
pub use note_type::NoteType;

mod nullifier;
pub use nullifier::Nullifier;

mod origin;
pub use origin::{NoteInclusionProof, NoteOrigin};

mod recipient;
pub use recipient::NoteRecipient;

mod script;
pub use script::NoteScript;

// CONSTANTS
// ================================================================================================

/// The depth of the leafs in the note Merkle tree used to commit to notes produced in a block.
/// This is equal `NOTE_TREE_DEPTH + 1`. In the kernel we do not authenticate leaf data directly
/// but rather authenticate hash(left_leaf, right_leaf).
pub const NOTE_LEAF_DEPTH: u8 = NOTE_TREE_DEPTH + 1;

// NOTE
// ================================================================================================

/// A note with all the data required for it to be consumed by executing it against the transaction
/// kernel.
///
/// Notes are created with a script, inputs, assets, and a serial number. Fungible and non-fungible
/// asset transfers are done by moving assets to the note's assets. The note's script determines the
/// conditions required for the note consumpution, i.e. the target account of a P2ID or conditions
/// of a SWAP, and the effects of the note. The serial number has a double duty of preventing double
/// spend, and providing unlikability to the consumer of a note. The note's inputs allow for
/// customization of its script.
///
/// To create a note, the kernel does not require all the information above, a user can create a
/// note only with the commitment to the script, inputs, the serial number, and the kernel only
/// verifies the source account has the assets necessary for the note creation. See [NoteRecipient]
/// for more details.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Note {
    assets: NoteAssets,
    metadata: NoteMetadata,
    recipient: NoteRecipient,

    id: NoteId,
    nullifier: Nullifier,
}

impl Note {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new note created with the specified parameters.
    pub fn new(assets: NoteAssets, metadata: NoteMetadata, recipient: NoteRecipient) -> Self {
        let id = NoteId::new(recipient.digest(), assets.commitment());
        let nullifier = Nullifier::new(
            recipient.script().hash(),
            recipient.inputs().commitment(),
            assets.commitment(),
            recipient.serial_num(),
        );

        Self {
            assets,
            metadata,
            id,
            recipient,
            nullifier,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the note's assets.
    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    /// Returns the note's metadata.
    pub fn metadata(&self) -> &NoteMetadata {
        &self.metadata
    }

    /// Returns the note's recipient.
    pub fn recipient(&self) -> &NoteRecipient {
        &self.recipient
    }

    /// Returns the note's unique identifier.
    ///
    /// This value is both an unique identifier and a commitment to the note.
    pub fn id(&self) -> NoteId {
        self.id
    }

    /// Returns the note's authentication hash.
    ///
    /// This value is used authenticate the note's presence in the note tree, it is computed as:
    ///
    /// > hash(note_id, note_metadata)
    ///
    pub fn authentication_hash(&self) -> Digest {
        Hasher::merge(&[self.id().inner(), Word::from(self.metadata()).into()])
    }

    /// Returns the note's nullifier.
    ///
    /// This is public data, used to prevent double spend.
    pub fn nullifier(&self) -> Nullifier {
        self.nullifier
    }

    /// Returns the note's recipient serial_num, the secret required to consume the note.
    pub fn serial_num(&self) -> Word {
        self.recipient.serial_num()
    }

    /// Returns the note's recipient script which locks the assets of this note.
    pub fn script(&self) -> &NoteScript {
        self.recipient.script()
    }

    /// Returns the note's recipient inputs which customizes the script's behavior.
    pub fn inputs(&self) -> &NoteInputs {
        self.recipient.inputs()
    }

    /// Returns the note's recipient digest, which commits to its details.
    ///
    /// This is the public data required to create a note.
    pub fn recipient_digest(&self) -> Digest {
        self.recipient.digest()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for Note {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let Self {
            assets,
            metadata,
            recipient,

            // These attributes don't have to be serialized, they can be re-computed from the rest
            // of the data
            id: _,
            nullifier: _,
        } = self;

        assets.write_into(target);
        metadata.write_into(target);
        recipient.write_into(target);
    }
}

impl Deserializable for Note {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let assets = NoteAssets::read_from(source)?;
        let metadata = NoteMetadata::read_from(source)?;
        let recipient = NoteRecipient::read_from(source)?;

        Ok(Self::new(assets, metadata, recipient))
    }
}
