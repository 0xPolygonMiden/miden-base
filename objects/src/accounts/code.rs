use super::{AccountError, Assembler, AssemblyContext, Digest, ModuleAst, Vec};
use crate::crypto::merkle::SimpleSmt;

// CONSTANTS
// ================================================================================================

/// The depth of the Merkle tree that is used to commit to the account's public interface.
const ACCOUNT_CODE_TREE_DEPTH: u8 = 8;

/// The maximum number of account interface procedures.
const MAX_ACCOUNT_PROCEDURES: usize = 2_usize.pow(ACCOUNT_CODE_TREE_DEPTH as u32);

// ACCOUNT CODE
// ================================================================================================

/// Describes the public interface of an account.
///
/// Account's public interface consists of a set of account procedures, each procedure being a Miden
/// VM program. Thus, MAST root of each procedure commits to the underlying program. We commit to
/// the entire account interface by building a simple Merkle tree out of all procedure MAST roots.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountCode {
    #[cfg_attr(feature = "serde", serde(with = "serialization"))]
    module: ModuleAst,
    procedures: Vec<Digest>,
    procedure_tree: SimpleSmt,
}

impl AccountCode {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    pub const ACCOUNT_CODE_NAMESPACE_BASE: &'static str = "context::account";

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new definition of an account's interface compiled from the specified source code.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Compilation of the provided module fails.
    /// - The number of procedures exported from the provided module is greater than 256.
    pub fn new(account_module: ModuleAst, assembler: &Assembler) -> Result<Self, AccountError> {
        // compile the module and make sure the number of exported procedures is within the limit
        let mut procedure_digests = assembler
            .compile_module(&account_module, None, &mut AssemblyContext::for_module(false))
            .map_err(AccountError::AccountCodeAssemblerError)?;

        if procedure_digests.len() > MAX_ACCOUNT_PROCEDURES {
            return Err(AccountError::AccountCodeTooManyProcedures {
                max: MAX_ACCOUNT_PROCEDURES,
                actual: procedure_digests.len(),
            });
        }

        // sort the procedure digests so that their order is stable
        procedure_digests.sort_by_key(|a| a.as_bytes());

        Ok(Self {
            procedure_tree: build_procedure_tree(&procedure_digests),
            module: account_module,
            procedures: procedure_digests,
        })
    }

    /// Returns a new definition of an account's interface instantiated from the provided
    /// module and list of procedure digests.
    ///
    /// # Safety
    /// This function assumes that the list of provided procedure digests resulted from the
    /// compilation of the provided module, but this is not checked.
    ///
    /// # Panics
    /// Panics if the number of procedures is greater than 256.
    pub unsafe fn from_parts(module: ModuleAst, procedures: Vec<Digest>) -> Self {
        assert!(procedures.len() <= MAX_ACCOUNT_PROCEDURES, "too many account procedures");
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
    pub fn procedure_tree(&self) -> &SimpleSmt {
        &self.procedure_tree
    }

    /// Returns the number of public interface procedures defined for this account.
    pub fn num_procedures(&self) -> usize {
        self.procedures.len()
    }

    /// Returns true if a procedure with the specified root is defined for this account.
    pub fn has_procedure(&self, root: Digest) -> bool {
        let root_bytes = root.as_bytes();
        self.procedures.binary_search_by(|r| r.as_bytes().cmp(&root_bytes)).is_ok()
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
        let root_bytes = root.as_bytes();
        self.procedures.binary_search_by(|x| x.as_bytes().cmp(&root_bytes)).ok()
    }
}

// SERIALIZATION
// ================================================================================================

#[cfg(feature = "serde")]
mod serialization {
    use assembly::ast::{AstSerdeOptions, ModuleAst};

    pub fn serialize<S>(module: &super::ModuleAst, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = module.to_bytes(AstSerdeOptions {
            serialize_imports: true,
        });

        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<super::ModuleAst, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = <Vec<u8> as serde::Deserialize>::deserialize(deserializer)?;

        ModuleAst::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

fn build_procedure_tree(procedures: &[Digest]) -> SimpleSmt {
    SimpleSmt::with_leaves(
        ACCOUNT_CODE_TREE_DEPTH,
        procedures
            .iter()
            .enumerate()
            .map(|(idx, p)| (idx as u64, p.into()))
            .collect::<Vec<_>>(),
    )
    .expect("failed to build procedure tree")
}
