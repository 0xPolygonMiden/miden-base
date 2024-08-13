use alloc::{string::ToString, vec::Vec};

use assembly::{Assembler, Compile, Library};
use vm_core::mast::MastForest;

use super::{
    AccountError, ByteReader, ByteWriter, Deserializable, DeserializationError, Digest, Felt,
    Hasher, Serializable,
};

pub mod procedure;
use procedure::AccountProcedureInfo;

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
    mast: MastForest,
    procedures: Vec<AccountProcedureInfo>,
    commitment: Digest,
}

impl AccountCode {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The maximum number of account interface procedures.
    pub const MAX_NUM_PROCEDURES: usize = u8::MAX as usize;

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Returns a new [AccountCode] instantiated from the provided [Library].
    ///
    /// All procedures exported from the provided library will become members of the account's
    /// public interface.
    ///
    /// # Errors
    /// Returns an error if the number of procedures exported from the provided library is smaller
    /// than 1 or greater than 256.
    pub fn new(library: Library) -> Result<Self, AccountError> {
        // extract procedure information from the library exports
        // TODO: currently, offsets for all procedures are set to 0; instead they should be read
        // from the Library metadata
        let mut procedures: Vec<AccountProcedureInfo> = Vec::new();
        for module in library.module_infos() {
            for proc_mast_root in module.procedure_digests() {
                procedures.push(AccountProcedureInfo::new(proc_mast_root, 0));
            }
        }

        // make sure the number of procedures is between 1 and 65535 (both inclusive)
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
            mast: library.into(),
        })
    }

    /// Returns a new [AccountCode] compiled from the provided source code using the specified
    /// assembler.
    ///
    /// All procedures exported from the provided code will become members of the account's
    /// public interface.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Compilation of the provided source code fails.
    /// - The number of procedures exported from the provided library is smaller than 1 or greater
    ///   than 256.
    pub fn compile(source_code: impl Compile, assembler: Assembler) -> Result<Self, AccountError> {
        let library = assembler
            .assemble_library([source_code])
            .map_err(|report| AccountError::AccountCodeAssemblyError(report.to_string()))?;
        Self::new(library)
    }

    /// Returns a new [AccountCode] deserialized from the provided bytes.
    ///
    /// # Errors
    /// Returns an error if account code deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AccountError> {
        Self::read_from_bytes(bytes).map_err(AccountError::AccountCodeDeserializationError)
    }

    /// Returns a new definition of an account's interface instantiated from the provided
    /// [MastForest] and a list of [AccountProcedureInfo]s.
    ///
    /// # Panics
    /// Panics if:
    /// - The number of procedures is smaller than 1 or greater than 256.
    /// - If some any of the provided procedures does not have a corresponding root in the
    ///   provided MAST forest.
    pub fn from_parts(mast: MastForest, procedures: Vec<AccountProcedureInfo>) -> Self {
        assert!(!procedures.is_empty(), "no account procedures");
        assert!(procedures.len() <= Self::MAX_NUM_PROCEDURES, "too many account procedures");

        // make sure all procedures are roots in the MAST forest
        for procedure in procedures.iter() {
            assert!(mast.find_procedure_root(*procedure.mast_root()).is_some());
        }

        Self {
            commitment: build_procedure_commitment(&procedures),
            procedures,
            mast,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn commitment(&self) -> Digest {
        self.commitment
    }

    /// Returns a reference to the [MastForest] backing this account code.
    pub fn mast(&self) -> &MastForest {
        &self.mast
    }

    /// Returns a reference to the account procedures.
    pub fn procedures(&self) -> &[AccountProcedureInfo] {
        &self.procedures
    }

    /// Returns an iterator over the procedure MAST roots of this account code.
    pub fn procedure_roots(&self) -> impl Iterator<Item = Digest> + '_ {
        self.procedures().iter().map(|procedure| *procedure.mast_root())
    }

    /// Returns the number of public interface procedures defined in this account code.
    pub fn num_procedures(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if a procedure with the specified MAST root is defined in this account code.
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

// CONVERSIONS
// ================================================================================================

impl From<AccountCode> for MastForest {
    fn from(code: AccountCode) -> Self {
        code.mast
    }
}

// EQUALITY
// ================================================================================================

impl PartialEq for AccountCode {
    fn eq(&self, other: &Self) -> bool {
        // TODO: consider checking equality based only on the set of procedures
        self.mast == other.mast && self.procedures == other.procedures
    }
}

impl Eq for AccountCode {}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountCode {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.mast.write_into(target);
        // since the number of procedures is guaranteed to be between 1 and 256, we can store the
        // number as a single byte - but we do have to subtract 1 to store 256 as 255.
        target.write_u8((self.procedures.len() - 1) as u8);
        target.write_many(self.procedures());
    }
}

impl Deserializable for AccountCode {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let module = MastForest::read_from(source)?;
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

    use super::{AccountCode, Deserializable, Serializable};
    use crate::accounts::code::build_procedure_commitment;

    #[test]
    fn test_serde() {
        let code = AccountCode::mock();
        let serialized = code.to_bytes();
        let deserialized = AccountCode::read_from_bytes(&serialized).unwrap();
        assert_eq!(deserialized, code)
    }

    #[test]
    fn test_account_code_procedure_commitment() {
        let code = AccountCode::mock();
        let procedure_commitment = build_procedure_commitment(code.procedures());
        assert_eq!(procedure_commitment, code.commitment())
    }
}
