use assembly::ast::AstSerdeOptions;

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
