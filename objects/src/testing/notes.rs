use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use assembly::Assembler;
use rand::Rng;

use crate::{
    accounts::AccountId,
    assembly::ProgramAst,
    assets::Asset,
    notes::{
        Note, NoteAssets, NoteInclusionProof, NoteInputs, NoteMetadata, NoteRecipient, NoteScript,
        NoteTag, NoteType,
    },
    Felt, NoteError, Word, ZERO,
};

pub const DEFAULT_NOTE_CODE: &str = "\
begin
end
";

#[derive(Debug, Clone)]
pub struct NoteBuilder {
    sender: AccountId,
    inputs: Vec<Felt>,
    assets: Vec<Asset>,
    note_type: NoteType,
    serial_num: Word,
    tag: NoteTag,
    code: String,
    proof: Option<NoteInclusionProof>,
    aux: Felt,
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
            note_type: NoteType::Public,
            serial_num,
            tag: 0.into(),
            code: DEFAULT_NOTE_CODE.to_string(),
            proof: None,
            aux: ZERO,
        }
    }

    /// Set the note's input to `inputs`.
    ///
    /// Note: This overwrite the inputs, the previous input values are discarded.
    pub fn note_inputs(
        mut self,
        inputs: impl IntoIterator<Item = Felt>,
    ) -> Result<Self, NoteError> {
        let validate = NoteInputs::new(inputs.into_iter().collect())?;
        self.inputs = validate.into();
        Ok(self)
    }

    pub fn add_assets(mut self, assets: impl IntoIterator<Item = Asset>) -> Self {
        self.assets.extend(assets);
        self
    }

    pub fn tag(mut self, tag: u32) -> Self {
        self.tag = tag.into();
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

    pub fn aux(mut self, aux: Felt) -> Self {
        self.aux = aux;
        self
    }

    pub fn build(self, assembler: &Assembler) -> Result<Note, NoteError> {
        let note_ast = ProgramAst::parse(&self.code).unwrap();
        let (note_script, _) = NoteScript::new(note_ast, assembler)?;
        let vault = NoteAssets::new(self.assets)?;
        let metadata = NoteMetadata::new(self.sender, self.note_type, self.tag, self.aux)?;
        let inputs = NoteInputs::new(self.inputs)?;
        let recipient = NoteRecipient::new(self.serial_num, note_script, inputs);
        Ok(Note::new(vault, metadata, recipient))
    }
}
