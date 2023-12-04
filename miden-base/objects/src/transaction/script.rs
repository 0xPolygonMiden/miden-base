use super::{Felt, Vec, Word};
use crate::{
    advice::{AdviceInputsBuilder, ToAdviceInputs},
    assembly::ProgramAst,
    errors::TransactionScriptError,
    utils::collections::BTreeMap,
    Digest,
};
use assembly::{Assembler, AssemblyContext};
use vm_core::code_blocks::CodeBlock;

/// A struct that represents a transaction script.
///
/// A transaction script is a program that is executed in a transaction after all input notes
/// have been executed.
///
/// The [TransactionScript] object is composed of:
/// - [code](TransactionScript::code): the transaction script source code.
/// - [hash](TransactionScript::hash): the hash of the compiled transaction script.
/// - [inputs](TransactionScript::inputs): a map of key, value inputs that are loaded into the
/// advice map such that the transaction script can access them.
#[derive(Clone, Debug)]
pub struct TransactionScript {
    code: ProgramAst,
    hash: Digest,
    inputs: BTreeMap<Digest, Vec<Felt>>,
}

impl TransactionScript {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of a [TransactionScript] with the provided script and inputs and the
    /// compiled script code block.
    pub fn new<T>(
        code: ProgramAst,
        inputs: T,
        assembler: &mut Assembler,
    ) -> Result<(Self, CodeBlock), TransactionScriptError>
    where
        T: IntoIterator<Item = (Word, Vec<Felt>)>,
    {
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

impl ToAdviceInputs for TransactionScript {
    fn to_advice_inputs<T: AdviceInputsBuilder>(&self, target: &mut T) {
        // insert the transaction script hash into the advice stack
        target.push_onto_stack(&**self.hash());

        // insert map inputs into the advice map
        for (hash, input) in self.inputs.iter() {
            target.insert_into_map(**hash, input.clone());
        }
    }
}
