use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use miden_objects::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    notes::{Note, NoteInclusionProof, NoteInputs, NoteScript},
    Felt, NoteError, Word,
};
use rand::Rng;

use super::TransactionKernel;

const DEFAULT_NOTE_CODE: &str = "\
begin
end
";

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct NoteBuilder {
    sender: AccountId,
    inputs: Vec<Felt>,
    assets: Vec<Asset>,
    serial_num: Word,
    tag: Felt,
    code: String,
    proof: Option<NoteInclusionProof>,
}

impl NoteBuilder {
    pub fn new<T: Rng>(sender: AccountId, mut rng: T) -> Self {
        let serial_num = [
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
            Felt::new(rng.gen()),
        ];

        Self {
            sender,
            inputs: vec![],
            assets: vec![],
            serial_num,
            tag: Felt::default(),
            code: DEFAULT_NOTE_CODE.to_string(),
            proof: None,
        }
    }

    pub fn note_inputs(mut self, inputs: Vec<Felt>) -> Result<Self, NoteError> {
        NoteInputs::new(inputs.to_vec())?;
        self.inputs = inputs;
        Ok(self)
    }

    pub fn add_asset(mut self, asset: Asset) -> Self {
        self.assets.push(asset);
        self
    }

    pub fn tag(mut self, tag: Felt) -> Self {
        self.tag = tag;
        self
    }

    pub fn code<S: AsRef<str>>(mut self, code: S) -> Self {
        self.code = code.as_ref().to_string();
        self
    }

    pub fn proof(mut self, proof: NoteInclusionProof) -> Self {
        self.proof = Some(proof);
        self
    }

    pub fn build(self) -> Result<Note, NoteError> {
        let assembler = TransactionKernel::assembler();
        let note_ast = ProgramAst::parse(&self.code).unwrap();
        let (note_script, _) = NoteScript::new(note_ast, &assembler)?;
        Note::new(note_script, &self.inputs, &self.assets, self.serial_num, self.sender, self.tag)
    }
}
