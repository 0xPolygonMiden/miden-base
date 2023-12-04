use super::{Assembler, AssemblyContext, CodeBlock, Digest, NoteError, ProgramAst};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteScript {
    hash: Digest,
    #[cfg_attr(feature = "serde", serde(with = "serialization"))]
    code: ProgramAst,
}

#[cfg(feature = "serde")]
mod serialization {
    use assembly::ast::AstSerdeOptions;

    pub fn serialize<S>(module: &super::ProgramAst, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = module.to_bytes(AstSerdeOptions {
            serialize_imports: true,
        });

        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::ProgramAst, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;

        super::ProgramAst::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
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
