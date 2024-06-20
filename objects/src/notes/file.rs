use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{Note, NoteDetails, NoteId, NoteInclusionProof, NoteTag};

// NOTE FILE
// ================================================================================================

/// A serialized representation of a note.
pub enum NoteFile {
    /// The note's details aren't known.
    NoteId(NoteId),
    /// The note has not yet been recorded on chain.
    ///
    /// An optional tag is included for note tracking.
    NoteDetails(NoteDetails, Option<NoteTag>),
    /// The note has been recorded on chain.
    NoteWithProof(Note, NoteInclusionProof),
}

impl From<NoteDetails> for NoteFile {
    fn from(details: NoteDetails) -> Self {
        NoteFile::NoteDetails(details, None)
    }
}

impl From<NoteId> for NoteFile {
    fn from(note_id: NoteId) -> Self {
        NoteFile::NoteId(note_id)
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteFile {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes("note".as_bytes());
        match self {
            NoteFile::NoteId(note_id) => {
                target.write_u8(0);
                note_id.write_into(target);
            },
            NoteFile::NoteDetails(details, tag) => {
                target.write_u8(1);
                details.write_into(target);
                tag.write_into(target);
            },
            NoteFile::NoteWithProof(note, proof) => {
                target.write_u8(2);
                note.write_into(target);
                proof.write_into(target);
            },
        }
    }
}

impl Deserializable for NoteFile {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let magic_value = source.read_string(4)?;
        if magic_value != "note" {
            return Err(DeserializationError::InvalidValue(format!(
                "Invalid note file marker: {magic_value}"
            )));
        }
        match source.read_u8()? {
            0 => Ok(NoteFile::NoteId(NoteId::read_from(source)?)),
            1 => {
                let details = NoteDetails::read_from(source)?;
                let tag = Option::<NoteTag>::read_from(source)?;
                Ok(NoteFile::NoteDetails(details, tag))
            },
            2 => {
                let note = Note::read_from(source)?;
                let proof = NoteInclusionProof::read_from(source)?;
                Ok(NoteFile::NoteWithProof(note, proof))
            },
            v => {
                Err(DeserializationError::InvalidValue(format!("Unknown variant {v} for NoteFile")))
            },
        }
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use assembly::{ast::ProgramAst, Assembler};
    use vm_core::{
        utils::{Deserializable, Serializable},
        Felt,
    };

    use crate::{
        accounts::{
            account_id::testing::{
                ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
                ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
            },
            AccountId,
        },
        assets::{Asset, FungibleAsset},
        notes::{
            Note, NoteAssets, NoteFile, NoteInclusionProof, NoteInputs, NoteMetadata,
            NoteRecipient, NoteScript, NoteTag, NoteType,
        },
        testing::notes::DEFAULT_NOTE_CODE,
    };

    fn create_example_note() -> Note {
        let faucet = AccountId::new_unchecked(Felt::new(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN));
        let target = AccountId::new_unchecked(Felt::new(
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ));

        let serial_num = [Felt::new(0), Felt::new(1), Felt::new(2), Felt::new(3)];
        let note_program_ast = ProgramAst::parse(DEFAULT_NOTE_CODE).unwrap();
        let (script, _) = NoteScript::new(note_program_ast, &Assembler::default()).unwrap();
        let note_inputs = NoteInputs::new(vec![target.into()]).unwrap();
        let recipient = NoteRecipient::new(serial_num, script, note_inputs);

        let asset = Asset::Fungible(FungibleAsset::new(faucet, 100).unwrap());
        let metadata =
            NoteMetadata::new(faucet, NoteType::Public, NoteTag::from(123), Felt::new(0)).unwrap();

        Note::new(NoteAssets::new(vec![asset]).unwrap(), metadata, recipient)
    }

    #[test]
    fn serialized_note_magic() {
        let note = create_example_note();
        let file = NoteFile::NoteId(note.id());
        let mut buffer = Vec::new();
        file.write_into(&mut buffer);

        let magic_value = &buffer[..4];
        assert_eq!(magic_value, b"note");
    }

    #[test]
    fn serialize_id() {
        let note = create_example_note();
        let file = NoteFile::NoteId(note.id());
        let mut buffer = Vec::new();
        file.write_into(&mut buffer);

        let file_copy = NoteFile::read_from_bytes(&buffer).unwrap();

        match file_copy {
            NoteFile::NoteId(note_id) => {
                assert_eq!(note.id(), note_id);
            },
            _ => panic!("Invalid note file variant"),
        }
    }

    #[test]
    fn serialize_details() {
        let note = create_example_note();
        let file = NoteFile::NoteDetails(note.details.clone(), Some(NoteTag::from(123)));
        let mut buffer = Vec::new();
        file.write_into(&mut buffer);

        let file_copy = NoteFile::read_from_bytes(&buffer).unwrap();

        match file_copy {
            NoteFile::NoteDetails(details, tag) => {
                assert_eq!(details, note.details);
                assert_eq!(tag, Some(NoteTag::from(123)));
            },
            _ => panic!("Invalid note file variant"),
        }
    }

    #[test]
    fn serialize_with_proof() {
        let note = create_example_note();
        let mock_inclusion_proof = NoteInclusionProof::new(
            Default::default(),
            Default::default(),
            Default::default(),
            0,
            Default::default(),
        )
        .unwrap();
        let file = NoteFile::NoteWithProof(note.clone(), mock_inclusion_proof.clone());
        let mut buffer = Vec::new();
        file.write_into(&mut buffer);

        let file_copy = NoteFile::read_from_bytes(&buffer).unwrap();

        match file_copy {
            NoteFile::NoteWithProof(note_copy, inclusion_proof_copy) => {
                assert_eq!(note, note_copy);
                assert_eq!(inclusion_proof_copy, mock_inclusion_proof);
            },
            _ => panic!("Invalid note file variant"),
        }
    }
}
