use super::{Assembler, AssemblyContext, CodeBlock, Digest, NoteError, ProgramAst};

#[derive(Debug, Clone)]
pub struct NoteScript {
    digest: Digest,
    code: ProgramAst,
}

impl NoteScript {
    pub fn new(
        code: ProgramAst,
        assembler: &mut Assembler,
        context: &mut AssemblyContext,
    ) -> Result<(Self, CodeBlock), NoteError> {
        let code_block = assembler
            .compile_in_context(code.clone(), context)
            .map_err(NoteError::ScriptCompilationError)?;
        Ok((
            Self {
                digest: code_block.hash(),
                code,
            },
            code_block,
        ))
    }

    pub fn hash(&self) -> Digest {
        self.digest
    }

    pub fn code(&self) -> &ProgramAst {
        &self.code
    }
}
