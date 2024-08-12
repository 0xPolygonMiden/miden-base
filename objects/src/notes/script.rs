use alloc::{string::ToString, vec::Vec};

use assembly::{Assembler, Compile};
use vm_core::{
    mast::{MastForest, MastNodeId},
    Program,
};

use super::{Digest, Felt};
use crate::{
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    NoteError,
};

// NOTE SCRIPT
// ================================================================================================

/// An executable program of a note.
///
/// A note's script represents a program which must be executed for a note to be consumed. As such
/// it defines the rules and side effects of consuming a given note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteScript {
    mast: MastForest,
    entrypoint: MastNodeId,
}

impl NoteScript {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteScript] instantiated from the provided program.
    pub fn new(code: Program) -> Self {
        Self {
            entrypoint: code.entrypoint(),
            mast: code.into(),
        }
    }

    /// Returns a new [NoteScript] compiled from the provided source code using the specified
    /// assembler.
    ///
    /// # Errors
    /// Returns an error if the compilation of the provided source code fails.
    pub fn compile(source_code: impl Compile, assembler: Assembler) -> Result<Self, NoteError> {
        let program = assembler
            .assemble_program(source_code)
            .map_err(|report| NoteError::NoteScriptAssemblyError(report.to_string()))?;
        Ok(Self::new(program))
    }

    /// Returns a new [NoteScript] deserialized from the provided bytes.
    ///
    /// # Errors
    /// Returns an error if note script deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NoteError> {
        Self::read_from_bytes(bytes).map_err(NoteError::NoteScriptDeserializationError)
    }

    /// Returns a new [NoteScript] instantiated from the provided components.
    ///
    /// # Panics
    /// Panics if the specified entrypoint is not in the provided MAST forest.
    pub fn from_parts(mast: MastForest, entrypoint: MastNodeId) -> Self {
        assert!(mast.get_node_by_id(entrypoint).is_some());
        Self { mast, entrypoint }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns MAST root of this note script.
    pub fn hash(&self) -> Digest {
        self.mast[self.entrypoint].digest()
    }

    /// Returns a reference to the [MastForest] backing this note script.
    pub fn mast(&self) -> &MastForest {
        &self.mast
    }
}

// CONVERSIONS INTO NOTE SCRIPT
// ================================================================================================

impl From<&NoteScript> for Vec<Felt> {
    fn from(script: &NoteScript) -> Self {
        let mut bytes = script.mast.to_bytes();
        let len = bytes.len();

        // Pad the data so that it can be encoded with u32
        let missing = if len % 4 > 0 { 4 - (len % 4) } else { 0 };
        bytes.resize(bytes.len() + missing, 0);

        let final_size = 2 + bytes.len();
        let mut result = Vec::with_capacity(final_size);

        // Push the length, this is used to remove the padding later
        result.push(Felt::from(script.entrypoint.as_u32()));
        result.push(Felt::new(len as u64));

        // A Felt can not represent all u64 values, so the data is encoded using u32.
        let mut encoded: &[u8] = &bytes;
        while encoded.len() >= 4 {
            let (data, rest) =
                encoded.split_first_chunk::<4>().expect("The length has been checked");
            let number = u32::from_le_bytes(*data);
            result.push(Felt::new(number.into()));

            encoded = rest;
        }

        result
    }
}

impl From<NoteScript> for Vec<Felt> {
    fn from(value: NoteScript) -> Self {
        (&value).into()
    }
}

// CONVERSIONS FROM NOTE SCRIPT
// ================================================================================================

impl TryFrom<&[Felt]> for NoteScript {
    type Error = DeserializationError;

    fn try_from(elements: &[Felt]) -> Result<Self, Self::Error> {
        if elements.len() < 2 {
            return Err(DeserializationError::UnexpectedEOF);
        }

        let entrypoint: u32 = elements[0].try_into().map_err(DeserializationError::InvalidValue)?;
        let len = elements[1].as_int();
        let mut data = Vec::with_capacity(elements.len() * 4);

        for &felt in &elements[2..] {
            let v: u32 = felt.try_into().map_err(DeserializationError::InvalidValue)?;
            data.extend(v.to_le_bytes())
        }
        data.shrink_to(len as usize);

        let mast = MastForest::read_from_bytes(&data)?;
        let entrypoint = MastNodeId::from_u32_safe(entrypoint, &mast)?;
        Ok(NoteScript::from_parts(mast, entrypoint))
    }
}

impl TryFrom<Vec<Felt>> for NoteScript {
    type Error = DeserializationError;

    fn try_from(value: Vec<Felt>) -> Result<Self, Self::Error> {
        value.as_slice().try_into()
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteScript {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.mast.write_into(target);
        target.write_u32(self.entrypoint.as_u32());
    }
}

impl Deserializable for NoteScript {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let mast = MastForest::read_from(source)?;
        let entrypoint = MastNodeId::from_u32_safe(source.read_u32()?, &mast)?;

        Ok(Self::from_parts(mast, entrypoint))
    }
}
