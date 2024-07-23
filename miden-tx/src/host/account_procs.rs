use alloc::string::ToString;
use miden_lib::transaction::TransactionKernelError;
use miden_objects::accounts::AccountProcedure;

use crate::error::TransactionHostError;

use super::{AdviceProvider, BTreeMap, Digest, Felt, ProcessState};

// ACCOUNT PROCEDURE INDEX MAP
// ================================================================================================

/// A map of proc_root |-> proc_index for all known procedures of an account interface.
pub struct AccountProcedureIndexMap(BTreeMap<Digest, u8>);

impl AccountProcedureIndexMap {
    /// Returns a new [AccountProcedureIndexMap] instantiated with account procedures present in
    /// the provided advice provider.
    pub fn new<A: AdviceProvider>(
        account_code_commitment: Digest,
        adv_provider: &A,
    ) -> Result<Self, TransactionHostError> {
        // get the account procedures from the advice_map
        let procs = adv_provider.get_mapped_values(&account_code_commitment).ok_or_else(|| {
            TransactionHostError::AccountProcedureIndexMapError(
                "Failed to get mapped values from the AdviceProvider".to_string(),
            )
        })?;

        let mut result = BTreeMap::new();

        // we skip procs[0] because it's the number of procedures
        for (proc_idx, proc_info) in procs[1..].chunks_exact(8).enumerate() {
            let proc_info_array: [Felt; 8] = proc_info.try_into().map_err(|_| {
                TransactionHostError::AccountProcedureIndexMapError(
                    "Invalid procedure info length".to_string(),
                )
            })?;

            let procedure = AccountProcedure::try_from(proc_info_array).map_err(|e| {
                TransactionHostError::AccountProcedureIndexMapError(format!(
                    "Failed to create AccountProcedure: {:?}",
                    e
                ))
            })?;

            let proc_idx = u8::try_from(proc_idx).map_err(|_| {
                TransactionHostError::AccountProcedureIndexMapError(format!(
                    "Invalid procedure index: {}",
                    proc_idx
                ))
            })?;

            result.insert(*procedure.mast_root(), proc_idx);
        }

        Ok(Self(result))
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
