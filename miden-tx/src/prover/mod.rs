use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::transaction::{
    InputNotes, PreparedTransaction, ProvenTransaction, TransactionWitness,
};
use miden_prover::prove;
pub use miden_prover::ProvingOptions;
use vm_processor::MemAdviceProvider;

use super::{TransactionHost, TransactionProverError};

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
        let (stack_inputs, advice_inputs) = transaction.get_kernel_inputs();
        let advice_provider: MemAdviceProvider = advice_inputs.into();
        let mut host = TransactionHost::new(advice_provider);

        let (stack_outputs, proof) =
            prove(transaction.program(), stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, _event_handler) = host.into_parts();
        let (_, map, _) = advice_provider.into_parts();
        let adv_map = map.into();
        let tx_outputs = TransactionKernel::parse_outputs(&stack_outputs, &adv_map)
            .map_err(TransactionProverError::TransactionResultError)?;

        let (_tx_program, tx_script, tx_inputs) = transaction.into_parts();

        Ok(ProvenTransaction::new(
            tx_inputs.account.id(),
            tx_inputs.account.hash(),
            tx_outputs.account.hash(),
            tx_inputs.input_notes.into(),
            tx_outputs.output_notes.into(),
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
        let (stack_inputs, advice_inputs) = tx_witness.get_kernel_inputs();

        let input_notes = match tx_witness.input_note_data() {
            Some(input_note_data) => {
                let nullifiers =
                    TransactionKernel::read_input_nullifiers_from(input_note_data).unwrap();
                InputNotes::new(nullifiers).unwrap()
            },
            None => InputNotes::default(),
        };

        let account_id = tx_witness.account_id();
        let initial_account_hash = tx_witness.initial_account_hash();
        let block_hash = tx_witness.block_hash();
        let tx_script_root = tx_witness.tx_script_root();

        let advice_provider: MemAdviceProvider = advice_inputs.into();
        let mut host = TransactionHost::new(advice_provider);
        let (stack_outputs, proof) =
            prove(tx_witness.program(), stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, _event_handler) = host.into_parts();
        let (_, map, _) = advice_provider.into_parts();
        let tx_outputs = TransactionKernel::parse_outputs(&stack_outputs, &map.into())
            .map_err(TransactionProverError::TransactionResultError)?;

        Ok(ProvenTransaction::new(
            account_id,
            initial_account_hash,
            tx_outputs.account.hash(),
            input_notes,
            tx_outputs.output_notes.into(),
            tx_script_root,
            block_hash,
            proof,
        ))
    }
}
