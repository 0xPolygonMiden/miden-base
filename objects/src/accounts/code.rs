use alloc::vec::Vec;

use assembly::ast::AstSerdeOptions;

use super::{
    AccountError, Assembler, AssemblyContext, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, ModuleAst, Serializable,
};
use crate::crypto::merkle::SimpleSmt;

// CONSTANTS
// ================================================================================================

/// Default serialization options for account code AST.
const MODULE_SERDE_OPTIONS: AstSerdeOptions = AstSerdeOptions::new(true);

/// The depth of the Merkle tree that is used to commit to the account's public interface.
pub const PROCEDURE_TREE_DEPTH: u8 = 8;

// ACCOUNT CODE
// ================================================================================================

/// A public interface of an account.
///
/// Account's public interface consists of a set of account procedures, each procedure being a Miden
/// VM program. Thus, MAST root of each procedure commits to the underlying program. We commit to
/// the entire account interface by building a simple Merkle tree out of all procedure MAST roots.
#[derive(Debug, Clone)]
pub struct AccountCode {
    module: ModuleAst,
    procedures: Vec<Digest>,
    procedure_tree: SimpleSmt<PROCEDURE_TREE_DEPTH>,
}

impl AccountCode {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The depth of the Merkle tree that is used to commit to the account's public interface.
    pub const PROCEDURE_TREE_DEPTH: u8 = PROCEDURE_TREE_DEPTH;

    /// The maximum number of account interface procedures.
    pub const MAX_NUM_PROCEDURES: usize = 2_usize.pow(Self::PROCEDURE_TREE_DEPTH as u32);

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new definition of an account's interface compiled from the specified source code.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Compilation of the provided module fails.
    /// - The number of procedures exported from the provided module is smaller than 1 or greater
    ///   than 256.
    pub fn new(module: ModuleAst, assembler: &Assembler) -> Result<Self, AccountError> {
        // compile the module and make sure the number of exported procedures is within the limit
        let procedures = assembler
            .compile_module(&module, None, &mut AssemblyContext::for_module(false))
            .map_err(AccountError::AccountCodeAssemblerError)?;

        // make sure the number of procedures is between 1 and 256 (both inclusive)
        if procedures.is_empty() {
            return Err(AccountError::AccountCodeNoProcedures);
        } else if procedures.len() > Self::MAX_NUM_PROCEDURES {
            return Err(AccountError::AccountCodeTooManyProcedures {
                max: Self::MAX_NUM_PROCEDURES,
                actual: procedures.len(),
            });
        }

        Ok(Self {
            procedure_tree: build_procedure_tree(&procedures),
            module,
            procedures,
        })
    }

    /// Returns a new definition of an account's interface instantiated from the provided
    /// module and list of procedure digests.
    ///
    /// **Note**: this function assumes that the list of provided procedure digests results from
    /// the compilation of the provided module, but this is not checked.
    ///
    /// # Panics
    /// Panics if the number of procedures is smaller than 1 or greater than 256.
    pub fn from_parts(module: ModuleAst, procedures: Vec<Digest>) -> Self {
        assert!(!procedures.is_empty(), "no account procedures");
        assert!(procedures.len() <= Self::MAX_NUM_PROCEDURES, "too many account procedures");
        Self {
            procedure_tree: build_procedure_tree(&procedures),
            module,
            procedures,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn root(&self) -> Digest {
        self.procedure_tree().root()
    }

    /// Returns a reference to the [ModuleAst] backing the [AccountCode].
    pub fn module(&self) -> &ModuleAst {
        &self.module
    }

    /// Returns a reference to the account procedure digests.
    pub fn procedures(&self) -> &[Digest] {
        &self.procedures
    }

    /// Returns a reference to the procedure tree.
    pub fn procedure_tree(&self) -> &SimpleSmt<PROCEDURE_TREE_DEPTH> {
        &self.procedure_tree
    }

    /// Returns the number of public interface procedures defined for this account.
    pub fn num_procedures(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if a procedure with the specified root is defined for this account.
    pub fn has_procedure(&self, root: Digest) -> bool {
        self.procedures.contains(&root)
    }

    /// Returns a procedure digest for the procedure with the specified index.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get_procedure_by_index(&self, index: usize) -> Digest {
        self.procedures[index]
    }

    /// Returns the procedure index for the procedure with the specified root or None if such
    /// procedure is not defined for this account.
    pub fn get_procedure_index_by_root(&self, root: Digest) -> Option<usize> {
        self.procedures.iter().position(|r| r == &root)
    }
}

// EQUALITY
// ================================================================================================

impl PartialEq for AccountCode {
    fn eq(&self, other: &Self) -> bool {
        // TODO: consider checking equality based only on the set of procedures
        self.module == other.module && self.procedures == other.procedures
    }
}

impl Eq for AccountCode {}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountCode {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.module.write_into(target, MODULE_SERDE_OPTIONS);
        self.module.write_source_locations(target);
        // since the number of procedures is guaranteed to be between 1 and 256, we can store the
        // number as a single byte - but we do have to subtract 1 to store 256 as 255.
        target.write_u8((self.procedures.len() - 1) as u8);
        target.write_many(self.procedures());
    }
}

impl Deserializable for AccountCode {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let mut module = ModuleAst::read_from(source, MODULE_SERDE_OPTIONS)?;
        module.load_source_locations(source)?;
        let num_procedures = (source.read_u8()? as usize) + 1;
        let procedures = source.read_many::<Digest>(num_procedures)?;

        Ok(Self::from_parts(module, procedures))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_procedure_tree(procedures: &[Digest]) -> SimpleSmt<PROCEDURE_TREE_DEPTH> {
    // order the procedure digests to achieve a reproducible tree
    let procedures = {
        let mut procedures = procedures.to_vec();
        procedures.sort_by_key(|a| a.as_bytes());
        procedures
    };

    SimpleSmt::<PROCEDURE_TREE_DEPTH>::with_leaves(
        procedures
            .iter()
            .enumerate()
            .map(|(idx, p)| (idx as u64, p.into()))
            .collect::<Vec<_>>(),
    )
    .expect("failed to build procedure tree")
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{AccountCode, Assembler, Deserializable, ModuleAst, Serializable};

    #[test]
    fn serialize_code() {
        let source = "
            export.foo
                push.1 push.2 mul
            end

            export.bar
                push.1 push.2 add
            end
        ";

        // build account code from source
        let module = ModuleAst::parse(source).unwrap();
        let assembler = Assembler::default();
        let code1 = AccountCode::new(module, &assembler).unwrap();

        // serialize and deserialize the code; make sure deserialized version matches the original
        let bytes = code1.to_bytes();
        let code2 = AccountCode::read_from_bytes(&bytes).unwrap();
        assert_eq!(code1, code2)
    }
}
