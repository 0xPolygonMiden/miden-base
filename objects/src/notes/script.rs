use super::{Assembler, AssemblyContext, CodeBlock, Digest, NoteError, ProgramAst};
use crate::utils::serde::{ByteReader, ByteWriter, Deserializable, Serializable};
use assembly::ast::AstSerdeOptions;
use vm_processor::DeserializationError;

// CONSTANTS
// ================================================================================================

/// Default serialization options for script code AST.
const CODE_SERDE_OPTIONS: AstSerdeOptions = AstSerdeOptions::new(true);

// NOTE SCRIPT
// ================================================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteScript {
    hash: Digest,
    code: ProgramAst,
}

impl NoteScript {
    pub fn new(code: ProgramAst, assembler: &Assembler) -> Result<(Self, CodeBlock), NoteError> {
        let code_block = assembler
            .compile_in_context(&code, &mut AssemblyContext::for_program(Some(&code)))
            .map_err(NoteError::ScriptCompilationError)?;
        Ok((
            Self {
                hash: code_block.hash(),
                code,
            },
            code_block,
        ))
    }

    pub fn hash(&self) -> Digest {
        self.hash
    }

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

        Ok(Self { hash, code })
    }
}
