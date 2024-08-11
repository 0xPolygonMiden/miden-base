use alloc::{collections::BTreeMap, sync::Arc};
use core::cell::RefCell;

use miden_lib::{transaction::TransactionKernel, MidenLib, StdLibrary};
use miden_objects::{
    assembly::mast::MastForest,
    transaction::{TransactionArgs, TransactionInputs},
    Digest,
};
use vm_processor::MastForestStore;

pub struct TransactionMastStore {
    mast_forests: RefCell<BTreeMap<Digest, Arc<MastForest>>>,
}

#[allow(clippy::new_without_default)]
impl TransactionMastStore {
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

    fn insert(&self, mast_forest: Arc<MastForest>) {
        let mut mast_forests = self.mast_forests.borrow_mut();

        // only register procedures that are local to this forest
        for proc_digest in mast_forest.local_procedure_digests() {
            mast_forests.insert(proc_digest, mast_forest.clone());
        }
    }
}

impl MastForestStore for TransactionMastStore {
    fn get(&self, procedure_hash: &Digest) -> Option<Arc<MastForest>> {
        self.mast_forests.borrow().get(procedure_hash).cloned()
    }
}
