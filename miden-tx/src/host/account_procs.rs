use alloc::string::ToString;

use miden_lib::transaction::TransactionKernelError;
use miden_objects::accounts::{AccountCode, AccountProcedureInfo};

use super::{AdviceProvider, BTreeMap, Digest, Felt, ProcessState};
use crate::errors::TransactionHostError;

// ACCOUNT PROCEDURE INDEX MAP
// ================================================================================================

/// A map of proc_root |-> proc_index for all known procedures of an account interface.
pub struct AccountProcedureIndexMap(BTreeMap<Digest, u8>);

impl AccountProcedureIndexMap {
    /// Returns a new [AccountProcedureIndexMap] instantiated with account procedures present in
    /// the provided advice provider.
    pub fn new(
        account_code_commitment: Digest,
        adv_provider: &impl AdviceProvider,
    ) -> Result<Self, TransactionHostError> {
        // get the account procedures from the advice_map
        let proc_data =
            adv_provider.get_mapped_values(&account_code_commitment).ok_or_else(|| {
                TransactionHostError::AccountProcedureIndexMapError(
                    "Failed to read account procedure data from the advice provider".to_string(),
                )
            })?;

        let mut result = BTreeMap::new();

        // sanity checks

        // check that there are procedures in the account code
        if proc_data.is_empty() {
            return Err(TransactionHostError::AccountProcedureIndexMapError(
                "The account code does not contain any procedures.".to_string(),
            ));
        }

        let num_procs = proc_data[0].as_int() as usize;

        // check that the account code does not contain too many procedures
        if num_procs > AccountCode::MAX_NUM_PROCEDURES {
            return Err(TransactionHostError::AccountProcedureIndexMapError(
                "The account code contains too many procedures.".to_string(),
            ));
        }

        // check that the stored number of procedures matches the length of the procedures array
        if num_procs * AccountProcedureInfo::NUM_ELEMENTS_PER_PROC != proc_data.len() - 1 {
            return Err(TransactionHostError::AccountProcedureIndexMapError(
                "Invalid number of procedures.".to_string(),
            ));
        }

        // we skip proc_data[0] because it's the number of procedures
        for (proc_idx, proc_info) in proc_data[1..]
            .chunks_exact(AccountProcedureInfo::NUM_ELEMENTS_PER_PROC)
            .enumerate()
        {
            let proc_info_array: [Felt; AccountProcedureInfo::NUM_ELEMENTS_PER_PROC] =
                proc_info.try_into().expect("Failed conversion into procedure info array.");

            let procedure = AccountProcedureInfo::try_from(proc_info_array).map_err(|e| {
                TransactionHostError::AccountProcedureIndexMapError(format!(
                    "Failed to create AccountProcedureInfo: {:?}",
                    e
                ))
            })?;

            let proc_idx = u8::try_from(proc_idx).expect("Invalid procedure index.");

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
    pub fn get_proc_index(
        &self,
        process: &impl ProcessState,
    ) -> Result<u8, TransactionKernelError> {
        let proc_root = process.get_stack_word(0).into();

        self.0
            .get(&proc_root)
            .cloned()
            .ok_or(TransactionKernelError::UnknownAccountProcedure(proc_root))
    }
}
