use alloc::{collections::BTreeMap, vec::Vec};

use super::{Digest, Felt, Word};
use crate::{
    assembly::{Assembler, AssemblyContext, ProgramAst},
    notes::NoteId,
    vm::CodeBlock,
    TransactionScriptError,
};

// TRANSACTION ARGS
// ================================================================================================

/// A struct that represents optional transaction arguments.
///
/// Transaction arguments consist of:
/// - Transaction script: a program that is executed in a transaction after all input notes
///   scripts have been executed.
/// - Note arguments: data put onto the the stack right before a note script is executed. These
///   are different from note inputs, as the user executing the transaction can specify arbitrary
///   note args.
#[derive(Clone, Debug, Default)]
pub struct TransactionArgs {
    tx_script: Option<TransactionScript>,
    note_args: BTreeMap<NoteId, Word>,
}

impl TransactionArgs {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns new [TransactionArgs] instantiated with the provided transaction script and note
    /// arguments.
    pub fn new(
        tx_script: Option<TransactionScript>,
        note_args: Option<BTreeMap<NoteId, Word>>,
    ) -> Self {
        Self {
            tx_script,
            note_args: note_args.unwrap_or_default(),
        }
    }

    /// Returns new [TransactionArgs] instantiated with the provided transaction script.
    pub fn with_tx_script(tx_script: TransactionScript) -> Self {
        Self {
            tx_script: Some(tx_script),
            note_args: BTreeMap::default(),
        }
    }

    /// Returns new [TransactionArgs] instantiated with the provided note arguments.
    pub fn with_note_args(not_args: BTreeMap<NoteId, Word>) -> Self {
        Self { tx_script: None, note_args: not_args }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }

    /// Returns a reference to a specific note argument.
    pub fn get_note_args(&self, note_id: NoteId) -> Option<&Word> {
        self.note_args.get(&note_id)
    }
}

// TRANSACTION SCRIPT
// ================================================================================================

/// A struct that represents a transaction script.
///
/// A transaction script is a program that is executed in a transaction after all input notes
/// have been executed.
///
/// The [TransactionScript] object is composed of:
/// - [code](TransactionScript::code): the transaction script source code.
/// - [hash](TransactionScript::hash): the hash of the compiled transaction script.
/// - [inputs](TransactionScript::inputs): a map of key, value inputs that are loaded into the
///   advice map such that the transaction script can access them.
#[derive(Clone, Debug)]
pub struct TransactionScript {
    code: ProgramAst,
    hash: Digest,
    inputs: BTreeMap<Digest, Vec<Felt>>,
}

impl TransactionScript {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of a [TransactionScript] with the provided script and inputs and the
    /// compiled script code block.
    ///
    /// # Errors
    /// Returns an error if script compilation fails.
    pub fn new<T: IntoIterator<Item = (Word, Vec<Felt>)>>(
        code: ProgramAst,
        inputs: T,
        assembler: &Assembler,
    ) -> Result<(Self, CodeBlock), TransactionScriptError> {
        let code_block = assembler
            .compile_in_context(&code, &mut AssemblyContext::for_program(Some(&code)))
            .map_err(TransactionScriptError::ScriptCompilationError)?;
        Ok((
            Self {
                code,
                hash: code_block.hash(),
                inputs: inputs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            },
            code_block,
        ))
    }

    /// Returns a new instance of a [TransactionScript] instantiated from the provided components.
    ///
    /// Note: this constructor does not verify that a compiled code in fact results in the provided
    /// hash.
    pub fn from_parts<T: IntoIterator<Item = (Word, Vec<Felt>)>>(
        code: ProgramAst,
        hash: Digest,
        inputs: T,
    ) -> Result<Self, TransactionScriptError> {
        Ok(Self {
            code,
            hash,
            inputs: inputs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the code.
    pub fn code(&self) -> &ProgramAst {
        &self.code
    }

    /// Returns a reference to the code hash.
    pub fn hash(&self) -> &Digest {
        &self.hash
    }

    /// Returns a reference to the inputs.
    pub fn inputs(&self) -> &BTreeMap<Digest, Vec<Felt>> {
        &self.inputs
    }
}
