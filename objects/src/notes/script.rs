use super::{Digest, NoteError};
use assembly::ProgramAst;

#[derive(Debug, Eq, PartialEq)]
pub struct NoteScript {
    hash: Digest,
    code: ProgramAst,
}

impl NoteScript {
    pub fn new<S>(script_src: S) -> Result<Self, NoteError>
    where
        S: AsRef<str>,
    {
        let code = ProgramAst::parse(script_src.as_ref()).unwrap();
        // TODO: the code needs to be compiled with tx kernel and miden rollup library; we need
        // to do this to get the code hash and initialize the hash filed properly
        Ok(Self {
            hash: Digest::default(),
            code,
        })
    }

    pub fn hash(&self) -> Digest {
        self.hash
    }

    pub fn code(&self) -> &ProgramAst {
        &self.code
    }
}
