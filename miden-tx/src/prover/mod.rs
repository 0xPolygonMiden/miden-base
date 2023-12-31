use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::transaction::{InputNotes, ProvenTransaction, TransactionWitness};
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

    // TRANSACTION PROVER
    // --------------------------------------------------------------------------------------------

    /// Proves the provided [TransactionWitness] and returns a [ProvenTransaction].
    ///
    /// # Errors
    /// - If the consumed note data in the transaction witness is corrupt.
    /// - If the transaction program cannot be proven.
    /// - If the transaction result is corrupt.
    pub fn prove_transaction<T: Into<TransactionWitness>>(
        &self,
        transaction: T,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        let tx_witness: TransactionWitness = transaction.into();

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
