use alloc::{collections::BTreeMap, sync::Arc};
use core::cell::RefCell;

use miden_lib::{transaction::TransactionKernel, MidenLib, StdLibrary};
use miden_objects::{
    accounts::AccountCode, assembly::mast::MastForest, notes::NoteScript,
    transaction::TransactionScript, Digest,
};
use vm_processor::MastForestStore;

pub struct TransactionMastStore {
    mast_forests: RefCell<BTreeMap<Digest, Arc<MastForest>>>,
}

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

    pub fn load_account(&self, code: AccountCode) {
        let mast_forest = Arc::new(code.into());
        self.insert(mast_forest);
    }

    pub fn load_note_script(&self, script: &NoteScript) {
        let mast_forest = Arc::new(script.mast().clone());
        self.insert(mast_forest);
    }

    pub fn load_tx_script(&self, script: &TransactionScript) {
        let mast_forest = Arc::new(script.mast().clone());
        self.insert(mast_forest);
    }

    fn insert(&self, mast_forest: Arc<MastForest>) {
        let mut mast_forests = self.mast_forests.borrow_mut();
        for proc_digest in mast_forest.procedure_digests() {
            mast_forests.insert(proc_digest, mast_forest.clone());
        }
    }
}

impl MastForestStore for TransactionMastStore {
    fn get(&self, procedure_hash: &Digest) -> Option<Arc<MastForest>> {
        self.mast_forests.borrow().get(procedure_hash).cloned()
    }
}
