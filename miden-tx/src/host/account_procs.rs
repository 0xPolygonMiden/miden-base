use miden_lib::transaction::TransactionKernelError;

use super::{AdviceProvider, BTreeMap, Digest, Felt, ProcessState};

// ACCOUNT PROCEDURE INDEX MAP
// ================================================================================================

/// A map of proc_root |-> proc_index for all known procedures of an account interface.
pub struct AccountProcedureIndexMap(BTreeMap<Digest, u8>);

impl AccountProcedureIndexMap {
    /// Returns a new [AccountProcedureIndexMap] instantiated with account procedures present in
    /// the provided advice provider.
    ///
    pub fn new<A: AdviceProvider>(account_code_root: Digest, adv_provider: &A) -> Self {
        // get the account procedures from the advice_map
        let procs = adv_provider.get_mapped_values(&account_code_root).unwrap();

        let mut result = BTreeMap::new();

        for (proc_idx, proc_info) in procs[1..].chunks_exact(8).enumerate() {
            let root: [Felt; 4] = proc_info[0..4].try_into().expect("Slice with incorrect len.");
            result.insert(Digest::from(root), proc_idx.try_into().unwrap());
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
        self.0
            .get(&proc_root)
            .cloned()
            .ok_or(TransactionKernelError::UnknownAccountProcedure(proc_root))
    }
}
