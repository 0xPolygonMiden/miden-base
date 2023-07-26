use super::{
    Assembler, AssemblyContext, AssemblyContextType, CodeBlock, Digest, NoteError, ProgramAst,
};

#[derive(Debug, Clone)]
pub struct NoteScript {
    hash: Digest,
    code: ProgramAst,
}

impl NoteScript {
    pub fn new(code: ProgramAst, assembler: &Assembler) -> Result<(Self, CodeBlock), NoteError> {
        let code_block = assembler
            .compile_in_context(&code, &mut AssemblyContext::new(AssemblyContextType::Program))
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
