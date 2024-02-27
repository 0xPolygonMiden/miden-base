use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    notes::{NoteEnvelope, Nullifier},
    transaction::{InputNotes, ProvenTransaction, ProvenTransactionBuilder, TransactionWitness},
};
use miden_prover::prove;
pub use miden_prover::ProvingOptions;
use vm_processor::{Digest, MemAdviceProvider};

use super::{TransactionHost, TransactionProverError};

/// Transaction prover is a stateless component which is responsible for proving transactions.
///
/// Transaction prover exposes the `prove_transaction` method which takes a [TransactionWitness],
/// or anything that can be converted into a [TransactionWitness], and returns a [ProvenTransaction].
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

    /// Proves the provided transaction and returns a [ProvenTransaction].
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

        let input_notes: InputNotes<Nullifier> = (tx_witness.tx_inputs().input_notes()).into();

        let account_id = tx_witness.account().id();
        let initial_account_hash = tx_witness.account().hash();
        let block_hash = tx_witness.block_header().hash();
        let tx_script_root = tx_witness.tx_args().tx_script().map(|script| *script.hash());

        let advice_provider: MemAdviceProvider = advice_inputs.into();
        let mut host = TransactionHost::new(tx_witness.account().into(), advice_provider);
        let (stack_outputs, proof) =
            prove(tx_witness.program(), stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, _event_handler) = host.into_parts();
        let (_, map, _) = advice_provider.into_parts();
        let tx_outputs = TransactionKernel::parse_transaction_outputs(&stack_outputs, &map.into())
            .map_err(TransactionProverError::InvalidTransactionOutput)?;

        let initial_hash = if tx_witness.account().is_new() {
            Digest::default()
        } else {
            initial_account_hash
        };

        let builder = ProvenTransactionBuilder::new()
            .account_id(account_id)
            .initial_account_hash(initial_hash)
            .final_account_hash(tx_outputs.account.hash())
            .add_input_notes(input_notes)
            .add_output_notes(tx_outputs.output_notes.into_iter().map(NoteEnvelope::from))
            .block_ref(block_hash)
            .proof(proof);

        let builder = match tx_script_root {
            Some(tx_script_root) => builder.tx_script_root(tx_script_root),
            _ => builder,
        };

        Ok(builder.build()?)
    }
}
