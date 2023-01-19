use super::{AccountError, Digest};
use assembly::{parse_module, ModuleAst};
use crypto::merkle::MerkleTree;
use miden_core::Program; // TODO: we should be able to import it from the assembly crate

// ACCOUNT CODE
// ================================================================================================

/// Describes public interface of an account.
///
/// Account's public interface consists of a set of account methods, each method being a Miden VM
/// program. Thus, MAST root of each method commits to the underlying program. We commit to the
/// entire account interface by building a simple Merkle tree out of all method MAST roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountCode {
    method_tree: MerkleTree,
    module: ModuleAst,
    // methods: Vec<Program>, commented out because Program doesn't currently implement Eq and
    // PartialEq. Also, there might be a better way of describing a set of programs as they
    // might share a lot of common code blocks. In a way, we want something like a Program
    // struct but with many entry points.
}

impl AccountCode {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates and returns a new definition of an account's interface compiled from the specified
    /// source code.
    pub fn new(source: &str) -> Result<Self, AccountError> {
        let _module_ast = parse_module(source)?;

        // TODO: compile module AST into a set of program MASTs. To do this we need to expose
        // a new method on the assembler to compile a module rather than a program (something
        // similar to Assembler::compile_module() but without internal parameters).
        //
        // Open question: how to initialize the assembler? Specifically, which libraries to
        // initialize it with. stdlib and midenlib are the two libraries we need for sure - but
        // how to handle accounts which rely on some user-defined libraries? i.e., should there
        // be a way to specify an "on-chain" library somehow?

        // TODO: build a Merkle tree out of MAST roots of compiled programs. The roots should
        // be sorted so that the same set of programs always resolves to the same root. If the
        // number of programs is not a power of two, the remaining leaves should be set to ZEROs.

        todo!()
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to an account's public interface.
    pub fn root(&self) -> Digest {
        self.method_tree.root().into()
    }

    /// Returns the number of public interface methods defined for this account.
    pub fn num_methods(&self) -> usize {
        todo!()
    }

    /// Returns true if a method with the specified root is defined for this account.
    pub fn has_method(&self, _root: Digest) -> bool {
        todo!()
    }

    /// Returns an account interface method at the specified index.
    ///
    /// # Panics
    /// Panics if the provided index is out of bounds.
    pub fn get_method_by_index(&self, _index: usize) -> &Program {
        todo!()
    }

    /// Returns an account interface method with the specified root or None if such method is not
    /// defined for this account.
    pub fn get_method_by_root(&self, _root: Digest) -> Option<&Program> {
        todo!()
    }
}
