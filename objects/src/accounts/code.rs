use alloc::vec::Vec;

use assembly::ast::AstSerdeOptions;
use vm_core::ZERO;

use super::{
    AccountError, Assembler, AssemblyContext, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, ModuleAst, Serializable,
};

// CONSTANTS
// ================================================================================================

/// Default serialization options for account code AST.
const MODULE_SERDE_OPTIONS: AstSerdeOptions = AstSerdeOptions::new(false);

/// The depth of the Merkle tree that is used to commit to the account's public interface.
pub const PROCEDURE_TREE_DEPTH: u8 = 8;

// ACCOUNT CODE
// ================================================================================================

/// A public interface of an account.
///
/// Account's public interface consists of a set of account procedures, each procedure being a Miden
/// VM program. Thus, MAST root of each procedure commits to the underlying program. We commit to
/// the entire account interface by building a sequential hash out of all procedure MAST roots.
#[derive(Debug, Clone)]
pub struct AccountCode {
    module: ModuleAst,
    procedures: Vec<(Digest, Felt)>,
    procedure_commitment: Digest,
}

impl AccountCode {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The maximum number of account interface procedures.
    pub const MAX_NUM_PROCEDURES: usize = u16::MAX as usize;

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

        let procedures: Vec<(Digest, Felt)> = procedures
            .into_iter()
            .enumerate()
            .map(|(i, proc)| (proc, Felt::new(i as u64)))
            .collect();

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
            procedure_commitment: build_procedure_commitment(&procedures),
            procedures,
            module,
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
    pub fn from_parts(module: ModuleAst, procedures: Vec<(Digest, Felt)>) -> Self {
        assert!(!procedures.is_empty(), "no account procedures");
        assert!(procedures.len() <= Self::MAX_NUM_PROCEDURES, "too many account procedures");
        Self {
            procedure_commitment: build_procedure_commitment(&procedures),
            procedures,
            module,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn root(&self) -> Digest {
        *self.procedure_commitment()
    }

    /// Returns a reference to the [ModuleAst] backing the [AccountCode].
    pub fn module(&self) -> &ModuleAst {
        &self.module
    }

    /// Returns a vector containing procedures as field elements.
    pub fn as_elements(&self) -> Vec<Felt> {
        procedures_as_elements(self.procedures())
    }

    /// Returns a reference to the account procedure digests.
    pub fn procedures(&self) -> &[(Digest, Felt)] {
        &self.procedures
    }

    /// Returns a reference to a commitment to an account's public interface.
    pub fn procedure_commitment(&self) -> &Digest {
        &self.procedure_commitment
    }

    /// Returns the number of public interface procedures defined for this account.
    pub fn num_procedures(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if a procedure with the specified root is defined for this account.
    pub fn has_procedure(&self, root: Digest) -> bool {
        self.procedures.iter().map(|(d, _)| *d).collect::<Vec<Digest>>().contains(&root)
    }

    /// Returns a procedure (digest, offset) pair for the procedure with the specified index.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get_procedure_by_index(&self, index: usize) -> (Digest, Felt) {
        self.procedures[index]
    }

    /// Returns the procedure index for the procedure with the specified root or None if such
    /// procedure is not defined for this account.
    pub fn get_procedure_index_by_root(&self, root: Digest) -> Option<usize> {
        self.procedures.iter().map(|(d, _)| d).position(|r| r == &root)
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
        // debug info (this includes module imports and source locations) is not serialized with account code
        self.module.write_into(target, MODULE_SERDE_OPTIONS);
        // since the number of procedures is guaranteed to be between 1 and 256, we can store the
        // number as a single byte - but we do have to subtract 1 to store 256 as 255.
        target.write_u8((self.procedures.len() - 1) as u8);
        target.write_many(self.procedures());
    }
}

impl Deserializable for AccountCode {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // debug info (this includes module imports and source locations) is not serialized with account code
        let module = ModuleAst::read_from(source, MODULE_SERDE_OPTIONS)?;
        let num_procedures = (source.read_u8()? as usize) + 1;
        let procedures = source.read_many::<(Digest, Felt)>(num_procedures)?;

        Ok(Self::from_parts(module, procedures))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn procedures_as_elements(procedures: &[(Digest, Felt)]) -> Vec<Felt> {
    let mut procedure_elements = Vec::with_capacity(procedures.len() * 2);
    for (proc_digest, storage_offset) in procedures {
        procedure_elements.extend_from_slice(proc_digest.as_elements());
        procedure_elements.extend_from_slice(&[*storage_offset, ZERO, ZERO, ZERO])
    }
    procedure_elements
}

fn build_procedure_commitment(procedures: &[(Digest, Felt)]) -> Digest {
    let elements = procedures_as_elements(procedures);
    Hasher::hash_elements(&elements)
}

// TESTING
// ================================================================================================

#[cfg(any(feature = "testing", test))]
pub mod testing {
    use super::{AccountCode, Assembler, ModuleAst};

    pub const CODE: &str = "
        export.foo
            push.1 push.2 mul
        end

        export.bar
            push.1 push.2 add
        end
    ";

    pub fn make_account_code() -> AccountCode {
        let mut module = ModuleAst::parse(CODE).unwrap();
        // clears are needed since they're not serialized for account code
        module.clear_imports();
        module.clear_locations();
        AccountCode::new(module, &Assembler::default()).unwrap()
    }
}
// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::{testing::*, AccountCode, Deserializable, Serializable};
    use crate::accounts::code::build_procedure_commitment;

    #[test]
    fn test_serde() {
        let code = make_account_code();
        let serialized = code.to_bytes();
        let deserialized = AccountCode::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, code)
    }

    #[test]
    fn test_account_code_procedure_commitment() {
        let code = make_account_code();

        let procedure_commitment = build_procedure_commitment(code.procedures());

        assert_eq!(&procedure_commitment, code.procedure_commitment())
    }
}
