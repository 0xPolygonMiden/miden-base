use alloc::vec::Vec;

use assembly::ast::AstSerdeOptions;

use super::{
    AccountError, Assembler, AssemblyContext, ByteReader, ByteWriter, Deserializable,
    DeserializationError, Digest, Felt, Hasher, ModuleAst, Serializable,
};

pub mod procedure;
use procedure::AccountProcedureInfo;

// CONSTANTS
// ================================================================================================

/// Default serialization options for account code AST.
const MODULE_SERDE_OPTIONS: AstSerdeOptions = AstSerdeOptions::new(false);

// ACCOUNT CODE
// ================================================================================================

/// A public interface of an account.
///
/// Account's public interface consists of a set of account procedures, each procedure being a
/// Miden VM program. Thus, MAST root of each procedure commits to the underlying program.
///
/// Each exported procedure is associated with a storage offset. This offset is applied to any
/// accesses made from within the procedure to the associated account's storage. For example, if
/// storage offset for a procedure is set ot 1, a call to the account::get_item(storage_slot=4)
/// made from this procedure would actually access storage slot with index 5.
///
/// We commit to the entire account interface by building a sequential hash of all procedure MAST
/// roots and associated storage_offset's. Specifically, each procedure contributes exactly 8 field
/// elements to the sequence of elements to be hashed. These elements are defined as follows:
///
/// ```text
/// [PROCEDURE_MAST_ROOT, storage_offset, 0, 0, 0]
/// ```
#[derive(Debug, Clone)]
pub struct AccountCode {
    module: ModuleAst,
    procedures: Vec<AccountProcedureInfo>,
    commitment: Digest,
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
    ///   than 65535.
    pub fn new(module: ModuleAst, assembler: &Assembler) -> Result<Self, AccountError> {
        // compile the module and make sure the number of exported procedures is within the limit
        let procedures = assembler
            .compile_module(&module, None, &mut AssemblyContext::for_module(false))
            .map_err(AccountError::AccountCodeAssemblerError)?;

        // TODO: Find way to input offset
        let procedures: Vec<AccountProcedureInfo> = procedures
            .into_iter()
            .enumerate()
            .map(|(i, proc)| AccountProcedureInfo::new(proc, i as u16))
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
            commitment: build_procedure_commitment(&procedures),
            procedures,
            module,
        })
    }

    /// Returns a new definition of an account's interface instantiated from the provided module
    /// and list of [AccountProcedureInfo]s.
    ///
    /// **Note**: this function assumes that the list of provided procedures results from the
    /// compilation of the provided module, but this is not checked.
    ///
    /// # Panics
    /// Panics if the number of procedures is smaller than 1 or greater than 65535.
    pub fn from_parts(module: ModuleAst, procedures: Vec<AccountProcedureInfo>) -> Self {
        assert!(!procedures.is_empty(), "no account procedures");
        assert!(procedures.len() <= Self::MAX_NUM_PROCEDURES, "too many account procedures");
        Self {
            commitment: build_procedure_commitment(&procedures),
            procedures,
            module,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns a reference to the [ModuleAst] backing this [AccountCode].
    pub fn module(&self) -> &ModuleAst {
        &self.module
    }

    /// Returns a reference to the account procedures.
    pub fn procedures(&self) -> &[AccountProcedureInfo] {
        &self.procedures
    }

    /// Returns an iterator over the procedure MAST roots of this [AccountCode].
    pub fn procedure_roots(&self) -> impl Iterator<Item = Digest> + '_ {
        self.procedures().iter().map(|procedure| *procedure.mast_root())
    }

    /// Returns the number of public interface procedures defined in this [AccountCode].
    pub fn num_procedures(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if a procedure with the specified MAST root is defined in this [AccountCode].
    pub fn has_procedure(&self, mast_root: Digest) -> bool {
        self.procedures.iter().any(|procedure| procedure.mast_root() == &mast_root)
    }

    /// Returns information about the procedure at the specified index.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get_procedure_by_index(&self, index: usize) -> &AccountProcedureInfo {
        &self.procedures[index]
    }

    /// Returns the procedure index for the procedure with the specified MAST root or None if such
    /// procedure is not defined in this [AccountCode].
    pub fn get_procedure_index_by_root(&self, root: Digest) -> Option<usize> {
        self.procedures
            .iter()
            .map(|procedure| procedure.mast_root())
            .position(|r| r == &root)
    }

    /// Converts procedure information in this [AccountCode] into a vector of field elements.
    ///
    /// This is done by first converting each procedure into exactly 8 elements as follows:
    /// ```text
    /// [PROCEDURE_MAST_ROOT, storage_offset, 0, 0, 0]
    /// ```
    /// And then concatenating the resulting elements into a single vector.
    pub fn as_elements(&self) -> Vec<Felt> {
        procedures_as_elements(self.procedures())
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
        // debug info (this includes module imports and source locations) is not serialized with
        // account code
        self.module.write_into(target, MODULE_SERDE_OPTIONS);
        // since the number of procedures is guaranteed to be between 1 and 256, we can store the
        // number as a single byte - but we do have to subtract 1 to store 256 as 255.
        target.write_u8((self.procedures.len() - 1) as u8);
        target.write_many(self.procedures());
    }
}

impl Deserializable for AccountCode {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        // debug info (this includes module imports and source locations) is not serialized with
        // account code
        let module = ModuleAst::read_from(source, MODULE_SERDE_OPTIONS)?;
        let num_procedures = (source.read_u8()? as usize) + 1;
        let procedures = source.read_many::<AccountProcedureInfo>(num_procedures)?;

        Ok(Self::from_parts(module, procedures))
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Converts given procedures into field elements
fn procedures_as_elements(procedures: &[AccountProcedureInfo]) -> Vec<Felt> {
    procedures
        .iter()
        .flat_map(|procedure| <[Felt; 8]>::from(procedure.clone()))
        .collect()
}

/// Computes the commitment to the given procedures
fn build_procedure_commitment(procedures: &[AccountProcedureInfo]) -> Digest {
    let elements = procedures_as_elements(procedures);
    Hasher::hash_elements(&elements)
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use assembly::{ast::ModuleAst, Assembler};

    use super::{AccountCode, Deserializable, Serializable};
    use crate::accounts::code::build_procedure_commitment;

    const CODE: &str = "
        export.foo
            push.1 push.2 mul
        end

        export.bar
            push.1 push.2 add
        end
    ";

    fn make_account_code() -> AccountCode {
        let mut module = ModuleAst::parse(CODE).unwrap();
        // clears are needed since they're not serialized for account code
        module.clear_imports();
        module.clear_locations();
        AccountCode::new(module, &Assembler::default()).unwrap()
    }

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

        assert_eq!(procedure_commitment, code.commitment())
    }
}
