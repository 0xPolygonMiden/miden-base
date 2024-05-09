use alloc::vec::Vec;

use assembly::ast::AstSerdeOptions;
use miden_crypto::Felt;

use super::{Assembler, AssemblyContext, CodeBlock, Digest, NoteError, ProgramAst};
use crate::utils::serde::{
    ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable,
};

// CONSTANTS
// ================================================================================================

/// Default serialization options for script code AST.
const CODE_SERDE_OPTIONS: AstSerdeOptions = AstSerdeOptions::new(true);

// NOTE SCRIPT
// ================================================================================================

/// An executable program of a note.
///
/// A note's script represents a program which must be executed for a note to be consumed. As such
/// it defines the rules and side effects of consuming a given note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteScript {
    hash: Digest,
    code: ProgramAst,
}

impl NoteScript {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [NoteScript] instantiated from the provided program and compiled with the
    /// provided assembler. The compiled code block is also returned.
    ///
    /// # Errors
    /// Returns an error if the compilation of the provided program fails.
    pub fn new(code: ProgramAst, assembler: &Assembler) -> Result<(Self, CodeBlock), NoteError> {
        let code_block = assembler
            .compile_in_context(&code, &mut AssemblyContext::for_program(Some(&code)))
            .map_err(NoteError::ScriptCompilationError)?;
        Ok((Self { hash: code_block.hash(), code }, code_block))
    }

    /// Returns a new [NoteScript] instantiated from the provided components.
    ///
    /// **Note**: this function assumes that the specified hash results from the compilation of the
    /// provided program, but this is not checked.
    pub fn from_parts(code: ProgramAst, hash: Digest) -> Self {
        Self { code, hash }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns MAST root of this note script.
    pub fn hash(&self) -> Digest {
        self.hash
    }

    /// Returns the AST of this note script.
    pub fn code(&self) -> &ProgramAst {
        &self.code
    }
}

// CONVERSIONS INTO NOTE SCRIPT
// ================================================================================================

impl From<&NoteScript> for Vec<Felt> {
    fn from(value: &NoteScript) -> Self {
        let mut bytes = value.code.to_bytes(AstSerdeOptions { serialize_imports: true });
        let len = bytes.len();

        // Pad the data so that it can be encoded with u32
        let missing = if len % 4 > 0 {
            4 - (len % 4)
        } else {
            0
        };
        bytes.resize(bytes.len() + missing, 0);

        let final_size = 5 + bytes.len();
        let mut result = Vec::with_capacity(final_size);

        // Push the length, this is used to remove the padding later
        result.extend(value.hash);
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

    fn try_from(value: &[Felt]) -> Result<Self, Self::Error> {
        if value.len() < 5 {
            return Err(DeserializationError::UnexpectedEOF);
        }

        let hash = Digest::new([value[0], value[1], value[2], value[3]]);
        let len = value[4].as_int();
        let mut data = Vec::with_capacity(value.len() * 4);

        for felt in &value[5..] {
            let v = u32::try_from(felt.as_int())
                .map_err(|v| DeserializationError::InvalidValue(format!("{v}")))?;
            data.extend(v.to_le_bytes())
        }
        data.shrink_to(len as usize);

        // TODO: validate the hash matches the code
        let code = ProgramAst::from_bytes(&data)?;
        Ok(NoteScript::from_parts(code, hash))
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
        self.hash.write_into(target);
        self.code.write_into(target, CODE_SERDE_OPTIONS);
    }
}

impl Deserializable for NoteScript {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let hash = Digest::read_from(source)?;
        let code = ProgramAst::read_from(source)?;

        Ok(Self::from_parts(code, hash))
    }
}
