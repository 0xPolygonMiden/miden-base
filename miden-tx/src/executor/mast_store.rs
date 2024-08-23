use alloc::{collections::BTreeMap, sync::Arc};
use core::cell::RefCell;

use miden_lib::{transaction::TransactionKernel, MidenLib, StdLibrary};
use miden_objects::{
    assembly::mast::MastForest,
    transaction::{TransactionArgs, TransactionInputs},
    Digest,
};
use vm_processor::MastForestStore;

// TRANSACTION MAST STORE
// ================================================================================================

/// A store for the code available during transaction execution.
///
/// Transaction MAST store contains a map between procedure MAST roots and [MastForest]s containing
/// MASTs for these procedures. The VM will request [MastForest]s from the store when it encounters
/// a procedure which it doesn't have the code for. Thus, to execute a program which makes
/// references to external procedures, the store must be loaded with [MastForest]s containing these
/// procedures.
pub struct TransactionMastStore {
    mast_forests: RefCell<BTreeMap<Digest, Arc<MastForest>>>,
}

#[allow(clippy::new_without_default)]
impl TransactionMastStore {
    /// Returns a new [TransactionMastStore] instantiated with the default libraries.
    ///
    /// The default libraries include:
    /// - Miden standard library (miden-stdlib).
    /// - Miden rollup library (miden-lib).
    /// - Transaction kernel.
    pub fn new() -> Self {
        let mast_forests = RefCell::new(BTreeMap::new());
        let store = Self { mast_forests };

        // load transaction kernel MAST forest
        let kernels_forest = Arc::new(TransactionKernel::kernel().into());
        store.insert(kernels_forest);

        // load miden-stdlib MAST forest
        let miden_stdlib_forest = Arc::new(StdLibrary::default().into());
        store.insert(miden_stdlib_forest);

        // load miden lib MAST forest
        let miden_lib_forest = Arc::new(MidenLib::default().into());
        store.insert(miden_lib_forest);

        store
    }

    /// Loads code required for executing a transaction with the specified inputs and args into
    /// this store.
    ///
    /// The loaded code includes:
    /// - Account code for the account specified in the provided [TransactionInputs].
    /// - Note scripts for all input notes in the provided [TransactionInputs].
    /// - Transaction script (if any) from the specified [TransactionArgs].
    pub fn load_transaction_code(&self, tx_inputs: &TransactionInputs, tx_args: &TransactionArgs) {
        // load account code
        self.insert(tx_inputs.account().code().mast().clone());

        // load note script MAST into the MAST store
        for note in tx_inputs.input_notes() {
            self.insert(note.note().script().mast().clone());
        }

        // load tx script MAST into the MAST store
        if let Some(tx_script) = tx_args.tx_script() {
            self.insert(tx_script.mast().clone());
        }
    }

    /// Registers all procedures of the provided [MastForest] with this store.
    pub fn insert(&self, mast_forest: Arc<MastForest>) {
        let mut mast_forests = self.mast_forests.borrow_mut();

        // only register procedures that are local to this forest
        for proc_digest in mast_forest.local_procedure_digests() {
            mast_forests.insert(proc_digest, mast_forest.clone());
        }
    }
}

// MAST FOREST STORE IMPLEMENTATION
// ================================================================================================

impl MastForestStore for TransactionMastStore {
    fn get(&self, procedure_hash: &Digest) -> Option<Arc<MastForest>> {
        self.mast_forests.borrow().get(procedure_hash).cloned()
    }
}
