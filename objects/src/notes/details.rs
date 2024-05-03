use miden_crypto::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    Word,
};
use vm_processor::DeserializationError;

use super::{Digest, NoteAssets, NoteId, NoteInputs, NoteRecipient, NoteScript, Nullifier};

// NOTE
// ================================================================================================

/// A note with all the data required for it to be consumed by executing it against the transaction
/// kernel.
///
/// Notes are created with a script, inputs, assets, and a serial number. Fungible and non-fungible
/// asset transfers are done by moving assets to the note's assets. The note's script determines the
/// conditions required for the note consumption, i.e. the target account of a P2ID or conditions
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
pub struct NoteDetails {
    assets: NoteAssets,
    recipient: NoteRecipient,
}

impl NoteDetails {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new note created with the specified parameters.
    pub fn new(assets: NoteAssets, recipient: NoteRecipient) -> Self {
        Self { assets, recipient }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the note's assets.
    pub fn assets(&self) -> &NoteAssets {
        &self.assets
    }

    /// Returns the note's recipient.
    pub fn recipient(&self) -> &NoteRecipient {
        &self.recipient
    }

    /// Returns the note's unique identifier.
    ///
    /// This value is both an unique identifier and a commitment to the note.
    pub fn id(&self) -> NoteId {
        NoteId::from(self)
    }

    /// Returns the note's nullifier.
    ///
    /// This is public data, used to prevent double spend.
    pub fn nullifier(&self) -> Nullifier {
        Nullifier::from(self)
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

    /// Decomposes note details into underlying assets and recipient.
    pub fn into_parts(self) -> (NoteAssets, NoteRecipient) {
        (self.assets, self.recipient)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteDetails {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let Self { assets, recipient } = self;

        assets.write_into(target);
        recipient.write_into(target);
    }
}

impl Deserializable for NoteDetails {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let assets = NoteAssets::read_from(source)?;
        let recipient = NoteRecipient::read_from(source)?;
        Ok(Self::new(assets, recipient))
    }
}
