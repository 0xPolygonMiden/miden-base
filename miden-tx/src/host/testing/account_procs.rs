use alloc::collections::BTreeMap;

use miden_lib::transaction::TransactionKernelError;
use miden_objects::accounts::AccountCode;

use super::{AdviceProvider, Digest, NodeIndex, ProcessState};

// ACCOUNT PROCEDURE INDEX MAP
// ================================================================================================

/// A map of proc_root |-> proc_index for all known procedures of an account interface.
pub struct AccountProcedureIndexMap(BTreeMap<Digest, u8>);

impl AccountProcedureIndexMap {
    /// Returns a new [AccountProcedureIndexMap] instantiated with account procedures present in
    /// the provided advice provider.
    ///
    /// This function assumes that the account procedure tree (or a part thereof) is loaded into the
    /// Merkle store of the provided advice provider.
    pub fn new<A: AdviceProvider>(account_code_root: Digest, adv_provider: &A) -> Self {
        // get the Merkle store with the procedure tree from the advice provider
        let proc_store = adv_provider.get_store_subset([account_code_root].iter());

        // iterate over all possible procedure indexes
        let mut result = BTreeMap::new();
        for i in 0..AccountCode::MAX_NUM_PROCEDURES {
            let index = NodeIndex::new(AccountCode::PROCEDURE_TREE_DEPTH, i as u64)
                .expect("procedure tree index is valid");
            // if the node at the current index does not exist, skip it and try the next node;this
            // situation is valid if not all account procedures are loaded into the advice provider
            if let Ok(proc_root) = proc_store.get_node(account_code_root, index) {
                // if we got an empty digest, this means we got to the end of the procedure list
                if proc_root == Digest::default() {
                    break;
                }
                result.insert(proc_root, i as u8);
            }
        }
        Self(result)
    }

    /// Returns index of the procedure whose root is currently at the top of the operand stack in
    /// the provided process.
    ///
    /// # Errors
    /// Returns an error if the procedure at the top of the operand stack is not present in this
    /// map.
    pub fn get_proc_index<S: ProcessState>(
        &self,
        process: &S,
    ) -> Result<u8, TransactionKernelError> {
        let proc_root = process.get_stack_word(0).into();
        // mock account method for testing from root context
        // TODO: figure out if we can get rid of this
        if proc_root == Digest::default() {
            return Ok(255);
        }
        self.0
            .get(&proc_root)
            .cloned()
            .ok_or(TransactionKernelError::UnknownAccountProcedure(proc_root))
    }
}
