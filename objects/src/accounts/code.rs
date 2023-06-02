use super::{
    AccountError, AccountId, Assembler, AssemblyContext, AssemblyContextType, Digest, LibraryPath,
    Module, ModuleAst,
};
use crypto::merkle::SimpleSmt;

// ACCOUNT CODE
// ================================================================================================

// CONSTANTS
// ------------------------------------------------------------------------------------------------

/// The depth of the Merkle tree that is used to commit to the account's public interface.
const ACCOUNT_CODE_TREE_DEPTH: u8 = 8;

/// Describes the public interface of an account.
///
/// Account's public interface consists of a set of account procedures, each procedure being a Miden
/// VM program. Thus, MAST root of each procedure commits to the underlying program. We commit to
/// the entire account interface by building a simple Merkle tree out of all procedure MAST roots.
#[derive(Debug, Clone)]
pub struct AccountCode {
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
    /// Creates and returns a new definition of an account's interface compiled from the specified
    /// source code.
    pub fn new(
        module_ast: ModuleAst,
        account_id: AccountId,
        assembler: &mut Assembler,
    ) -> Result<Self, AccountError> {
        let module = Module::new(
            LibraryPath::new(format!("{}_{}", Self::ACCOUNT_CODE_NAMESPACE_BASE, account_id))
                .expect("valid path"),
            module_ast,
        );

        let mut procedure_digests = assembler
            .compile_module(&module, &mut AssemblyContext::new(AssemblyContextType::Module))
            .map_err(AccountError::AccountCodeAsselmberError)?;
        procedure_digests.sort_by_key(|a| a.as_bytes());

        Ok(Self {
            procedure_tree: SimpleSmt::with_leaves(
                ACCOUNT_CODE_TREE_DEPTH,
                procedure_digests
                    .iter()
                    .enumerate()
                    .map(|(idx, p)| (idx as u64, p.into()))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
            module: module.ast,
            procedures: procedure_digests,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn root(&self) -> Digest {
        self.procedure_tree.root().into()
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
        // index must be wihtin range
        assert!(index < self.procedures.len());

        // Return digest for the procedure
        *self.procedures.get(index).unwrap()
    }

    /// Returns the procedure index for the procedure with the specified root or None if such
    /// procedure is not defined for this account.
    pub fn get_procedure_index_by_root(&self, root: Digest) -> Option<usize> {
        let root_bytes = root.as_bytes();
        self.procedures.binary_search_by(|x| x.as_bytes().cmp(&root_bytes)).ok()
    }
}
