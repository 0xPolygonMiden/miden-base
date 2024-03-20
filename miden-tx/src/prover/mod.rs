use miden_lib::transaction::{ToTransactionKernelInputs, TransactionKernel};
use miden_objects::{
    notes::Nullifier,
    transaction::{InputNotes, ProvenTransaction, ProvenTransactionBuilder, TransactionWitness},
};
use miden_prover::prove;
pub use miden_prover::ProvingOptions;
use vm_processor::MemAdviceProvider;

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
        let block_hash = tx_witness.block_header().hash();
        let tx_script_root = tx_witness.tx_args().tx_script().map(|script| *script.hash());

        let advice_provider: MemAdviceProvider = advice_inputs.into();
        let mut host = TransactionHost::new(tx_witness.account().into(), advice_provider);
        let (stack_outputs, proof) =
            prove(tx_witness.program(), stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, account_delta, output_notes) = host.into_parts();
        let (_, map, _) = advice_provider.into_parts();
        let tx_outputs =
            TransactionKernel::from_transaction_parts(&stack_outputs, &map.into(), output_notes)
                .map_err(TransactionProverError::InvalidTransactionOutput)?;

        let builder = ProvenTransactionBuilder::new(
            account_id,
            tx_witness.account().proof_init_hash(),
            tx_outputs.account.hash(),
            block_hash,
            proof,
        )
        .add_input_notes(input_notes)
        .add_output_notes(tx_outputs.output_notes.iter().cloned());

        let builder = match tx_script_root {
            Some(tx_script_root) => builder.tx_script_root(tx_script_root),
            _ => builder,
        };

        let builder = match account_id.is_on_chain() {
            true => builder.account_delta(account_delta),
            false => builder,
        };

        builder.build().map_err(TransactionProverError::ProvenTransactionError)
    }
}
