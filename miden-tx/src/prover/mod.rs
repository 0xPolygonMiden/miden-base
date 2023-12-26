use miden_objects::transaction::{
    PreparedTransaction, ProvenTransaction, TransactionOutputs, TransactionWitness,
};
use miden_prover::prove;
pub use miden_prover::ProvingOptions;
use vm_processor::MemAdviceProvider;

use super::{TransactionHost, TransactionProverError};
use crate::TryFromVmResult;

/// The [TransactionProver] is a stateless component which is responsible for proving transactions.
///
/// The [TransactionProver] exposes the `prove_transaction` method which takes a [TransactionWitness] and
/// produces a [ProvenTransaction].
pub struct TransactionProver {
    proof_options: ProvingOptions,
}

impl TransactionProver {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionProver] instance.
    pub fn new(proof_options: ProvingOptions) -> Self {
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
        let advice_provider: MemAdviceProvider = transaction.advice_provider_inputs().into();
        let mut host = TransactionHost::new(advice_provider);
        let (outputs, proof) = prove(
            transaction.program(),
            transaction.stack_inputs(),
            &mut host,
            self.proof_options.clone(),
        )
        .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, _event_handler) = host.into_parts();
        let (stack, map, store) = advice_provider.into_parts();
        let tx_outputs = TransactionOutputs::try_from_vm_result(&outputs, &stack, &map, &store)
            .map_err(TransactionProverError::TransactionResultError)?;

        let (_tx_program, tx_script, tx_inputs) = transaction.into_parts();

        Ok(ProvenTransaction::new(
            tx_inputs.account.id(),
            tx_inputs.account.hash(),
            tx_outputs.account.hash(),
            tx_inputs.input_notes.nullifiers().collect(),
            tx_outputs.output_notes.envelopes().collect(),
            tx_script.map(|tx_script| *tx_script.hash()),
            tx_inputs.block_header.hash(),
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
            .input_notes_info()
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

        let advice_provider: MemAdviceProvider = advice_witness.into();
        let mut host = TransactionHost::new(advice_provider);
        let (outputs, proof) =
            prove(&tx_program, stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, _event_handler) = host.into_parts();
        let (stack, map, store) = advice_provider.into_parts();
        let tx_outputs = TransactionOutputs::try_from_vm_result(&outputs, &stack, &map, &store)
            .map_err(TransactionProverError::TransactionResultError)?;

        Ok(ProvenTransaction::new(
            account_id,
            initial_account_hash,
            tx_outputs.account.hash(),
            consumed_notes_info,
            tx_outputs.output_notes.envelopes().collect(),
            tx_script_root,
            block_hash,
            proof,
        ))
    }
}
