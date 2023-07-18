use super::{MemAdviceProvider, ProvenTransaction, TransactionProverError, TransactionWitness};
use miden_objects::{
    transaction::{PreparedTransaction, TransactionOutputs},
    TryFromVmResult,
};
use miden_prover::{prove, ProofOptions};

/// The [TransactionProver] is a stateless component which is responsible for proving transactions.
///
/// The [TransactionProver] exposes the `prove_transaction` method which takes a [TransactionWitness] and
/// produces a [ProvenTransaction].
pub struct TransactionProver {
    proof_options: ProofOptions,
}

impl TransactionProver {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionProver] instance.
    pub fn new(proof_options: ProofOptions) -> Self {
        Self { proof_options }
    }

    /// Proves the provided [PreparedTransaction] and returns a [ProvenTransaction].
    ///
    /// # Errors
    /// - If the transaction program cannot be proven.
    /// - If the transaction result is corrupt.
    pub fn prove_prepared_transaction(
        &self,
        transaction: PreparedTransaction,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        // prove transaction program
        let mut advice_provider: MemAdviceProvider = transaction.advice_provider_inputs().into();
        let (outputs, proof) = prove(
            transaction.tx_program(),
            transaction.stack_inputs(),
            &mut advice_provider,
            self.proof_options.clone(),
        )
        .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let transaction_outputs =
            TransactionOutputs::try_from_vm_result(&outputs, &advice_provider)
                .map_err(TransactionProverError::TransactionResultError)?;
        let consumed_notes_info = transaction
            .consumed_notes()
            .notes()
            .iter()
            .map(|note| note.into())
            .collect::<Vec<_>>();

        let created_notes_info = transaction_outputs
            .created_notes
            .notes()
            .iter()
            .map(|note| note.into())
            .collect::<Vec<_>>();

        Ok(ProvenTransaction::new(
            transaction.account().id(),
            transaction.account().hash(),
            transaction_outputs.final_account_stub.0.hash(),
            consumed_notes_info,
            created_notes_info,
            transaction.tx_script_root(),
            transaction.tx_program().hash(),
            transaction.block_header().hash(),
            proof,
        ))
    }

    /// Proves the provided [TransactionWitness] and returns a [ProvenTransaction].
    ///
    /// # Errors
    /// - If the consumed note data in the transaction witness is corrupt.
    /// - If the transaction program cannot be proven.
    /// - If the transaction result is corrupt.
    pub fn prove_transaction_witness(
        &self,
        tx_witness: TransactionWitness,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        // extract required data from the transaction witness
        let stack_inputs = tx_witness.get_stack_inputs();
        let consumed_notes_info = tx_witness
            .consumed_notes_info()
            .map_err(TransactionProverError::CorruptTransactionWitnessConsumedNoteData)?;
        let (
            account_id,
            initial_account_hash,
            block_hash,
            _consumed_notes_hash,
            tx_script_root,
            tx_program,
            advice_witness,
        ) = tx_witness.into_parts();

        let mut advice_provider: MemAdviceProvider = advice_witness.into();
        let (outputs, proof) =
            prove(&tx_program, stack_inputs, &mut advice_provider, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let transaction_outputs =
            TransactionOutputs::try_from_vm_result(&outputs, &advice_provider)
                .map_err(TransactionProverError::TransactionResultError)?;
        let created_notes_info = transaction_outputs
            .created_notes
            .notes()
            .iter()
            .map(|note| note.into())
            .collect::<Vec<_>>();

        Ok(ProvenTransaction::new(
            account_id,
            initial_account_hash,
            transaction_outputs.final_account_stub.0.hash(),
            consumed_notes_info,
            created_notes_info,
            tx_script_root,
            tx_program.hash(),
            block_hash,
            proof,
        ))
    }
}
