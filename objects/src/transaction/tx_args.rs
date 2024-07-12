use alloc::{collections::BTreeMap, vec::Vec};
use core::ops::Deref;

use miden_crypto::merkle::InnerNodeInfo;
use vm_processor::{AdviceInputs, AdviceMap};

use super::{Digest, Felt, Word};
use crate::{
    assembly::{Assembler, AssemblyContext, ProgramAst},
    notes::{NoteDetails, NoteId},
    vm::CodeBlock,
    TransactionScriptError,
};

// TRANSACTION ARGS
// ================================================================================================

/// A struct that represents optional transaction arguments.
///
/// - Transaction script: a program that is executed in a transaction after all input notes
///   scripts have been executed.
/// - Note arguments: data put onto the stack right before a note script is executed. These
///   are different from note inputs, as the user executing the transaction can specify arbitrary
///   note args.
/// - Advice inputs: Provides data needed by the runtime, like the details of a public output note.
#[derive(Clone, Debug, Default)]
pub struct TransactionArgs {
    tx_script: Option<TransactionScript>,
    note_args: BTreeMap<NoteId, Word>,
    advice_inputs: AdviceInputs,
}

impl TransactionArgs {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns new [TransactionArgs] instantiated with the provided transaction script and note
    /// arguments.
    ///
    /// If tx_script is provided, this also adds all mappings from the transaction script inputs
    /// to the advice inputs' map.
    pub fn new(
        tx_script: Option<TransactionScript>,
        note_args: Option<BTreeMap<NoteId, Word>>,
        advice_map: AdviceMap,
    ) -> Self {
        let mut advice_inputs = AdviceInputs::default().with_map(advice_map);
        // add transaction script inputs to the advice inputs' map
        if let Some(ref tx_script) = tx_script {
            advice_inputs
                .extend_map(tx_script.inputs().iter().map(|(hash, input)| (*hash, input.clone())))
        }

        Self {
            tx_script,
            note_args: note_args.unwrap_or_default(),
            advice_inputs,
        }
    }

    /// Returns new [TransactionArgs] instantiated with the provided transaction script.
    pub fn with_tx_script(tx_script: TransactionScript) -> Self {
        Self::new(Some(tx_script), Some(BTreeMap::default()), AdviceMap::default())
    }

    /// Returns new [TransactionArgs] instantiated with the provided note arguments.
    pub fn with_note_args(note_args: BTreeMap<NoteId, Word>) -> Self {
        Self::new(None, Some(note_args), AdviceMap::default())
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

    /// Returns a reference to the args [AdviceInputs].
    pub fn advice_inputs(&self) -> &AdviceInputs {
        &self.advice_inputs
    }

    // STATE MUTATORS
    // --------------------------------------------------------------------------------------------

    /// Populates the advice inputs with the specified note details.
    ///
    /// The advice inputs' map is extended with the following keys:
    ///
    /// - recipient |-> recipient details (inputs_hash, script_hash, serial_num).
    /// - inputs_key |-> inputs, where inputs_key is computed by taking note inputs commitment and
    ///   adding ONE to its most significant element.
    /// - script_hash |-> script.
    pub fn add_expected_output_note<T: Deref<Target = NoteDetails>>(&mut self, note: &T) {
        let recipient = note.recipient();
        let inputs = note.inputs();
        let script = note.script();
        let script_encoded: Vec<Felt> = script.into();

        let new_elements = [
            (recipient.digest(), recipient.to_elements()),
            (inputs.commitment(), inputs.format_for_advice()),
            (script.hash(), script_encoded),
        ];

        self.advice_inputs.extend_map(new_elements);
    }

    /// Populates the advice inputs with the specified note details.
    ///
    /// The advice inputs' map is extended with the following keys:
    ///
    /// - recipient |-> recipient details (inputs_hash, script_hash, serial_num)
    /// - inputs_key |-> inputs, where inputs_key is computed by taking note inputs commitment and
    ///   adding ONE to its most significant element.
    /// - script_hash |-> script
    pub fn extend_expected_output_notes<T, L>(&mut self, notes: L)
    where
        L: IntoIterator<Item = T>,
        T: Deref<Target = NoteDetails>,
    {
        for note in notes {
            self.add_expected_output_note(&note);
        }
    }

    /// Extends the internal advice inputs' map with the provided key-value pairs.
    pub fn extend_advice_map<T: IntoIterator<Item = (Digest, Vec<Felt>)>>(&mut self, iter: T) {
        self.advice_inputs.extend_map(iter)
    }

    /// Extends the internal advice inputs' map with the provided key-value pairs.
    pub fn extend_merkle_store<I: Iterator<Item = InnerNodeInfo>>(&mut self, iter: I) {
        self.advice_inputs.extend_merkle_store(iter)
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
///   advice inputs' map such that the transaction script can access them.
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
