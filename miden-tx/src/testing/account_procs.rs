use alloc::collections::BTreeMap;
use std::panic;

use miden_lib::transaction::TransactionKernelError;
use miden_objects::accounts::{AccountCode, AccountProcedureInfo};
use vm_processor::{AdviceProvider, Digest, Felt, ProcessState};

// ACCOUNT PROCEDURE INDEX MAP
// ================================================================================================

/// A map of proc_root |-> proc_index for all known procedures of an account interface.
pub struct AccountProcedureIndexMap(BTreeMap<Digest, u16>);

impl AccountProcedureIndexMap {
    /// Returns a new [AccountProcedureIndexMap] instantiated with account procedures present in
    /// the provided advice provider.
    pub fn new<A: AdviceProvider>(account_code_commitment: Digest, adv_provider: &A) -> Self {
        // get the account procedures from the advice_map
        let procs = adv_provider
            .get_mapped_values(&account_code_commitment)
            .expect("Failed to read account procedure data from the advice provider");

        let mut result = BTreeMap::new();

        // sanity checks

        // check that there are procedures in the account code
        if procs.is_empty() {
            panic!("The account code does not contain any procedures.");
        }

        // check that the account code does not contain too many procedures
        if procs.len() > AccountCode::MAX_NUM_PROCEDURES {
            panic!("The account code contains too many procedures.");
        }

        // check that the stored number of procedures matches the length of the procedures array
        if procs[0].as_int() * 8 != procs.len() as u64 - 1 {
            panic!("Invalid number of procedures.")
        }

        // we skip procs[0] because it's the number of procedures
        for (proc_idx, proc_info) in procs[1..].chunks_exact(8).enumerate() {
            let proc_info_array: [Felt; 8] =
                proc_info.try_into().expect("Invalid procedure info length");

            let procedure = AccountProcedureInfo::try_from(proc_info_array)
                .expect("Failed to create AccountProcedure: {:?}");

            let proc_idx = u16::try_from(proc_idx).expect("Invalid procedure index");

            result.insert(*procedure.mast_root(), proc_idx);
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
    ) -> Result<u16, TransactionKernelError> {
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
