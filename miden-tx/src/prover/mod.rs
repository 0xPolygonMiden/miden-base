use alloc::{rc::Rc, vec::Vec};

use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::delta::AccountUpdateDetails,
    transaction::{OutputNote, ProvenTransaction, ProvenTransactionBuilder, TransactionWitness},
};
use miden_prover::prove;
pub use miden_prover::ProvingOptions;
use vm_processor::MemAdviceProvider;
use winter_maybe_async::maybe_async;

use super::{TransactionHost, TransactionProverError};
use crate::executor::TransactionMastStore;

// TRANSACTION PROVER TRAIT
// ================================================================================================

/// The [TransactionProver] trait defines the interface that transaction witness objects use to
/// prove transactions and generate a [ProvenTransaction].
pub trait TransactionProver {
    /// Proves the provided transaction and returns a [ProvenTransaction].
    ///
    /// # Errors
    /// - If the input note data in the transaction witness is corrupt.
    /// - If the transaction program cannot be proven.
    /// - If the transaction result is corrupt.
    #[maybe_async]
    fn prove(
        &self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError>;
}

// LOCAL TRANSACTION PROVER
// ------------------------------------------------------------------------------------------------

/// Local Transaction prover is a stateless component which is responsible for proving transactions.
///
/// Local Transaction Prover implements the [TransactionProver] trait.
pub struct LocalTransactionProver {
    mast_store: Rc<TransactionMastStore>,
    proof_options: ProvingOptions,
}

impl LocalTransactionProver {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [LocalTransactionProver] instance.
    pub fn new(proof_options: ProvingOptions) -> Self {
        Self {
            mast_store: Rc::new(TransactionMastStore::new()),
            proof_options,
        }
    }
}

impl Default for LocalTransactionProver {
    fn default() -> Self {
        Self { mast_store: Rc::new(TransactionMastStore::new()), proof_options: Default::default() }
    }
}

impl TransactionProver for LocalTransactionProver {
    #[maybe_async]
    fn prove(
        &self,
        transaction: impl Into<TransactionWitness>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        let tx_witness: TransactionWitness = transaction.into();
        let TransactionWitness { tx_inputs, tx_args, advice_witness } = tx_witness;

        let account = tx_inputs.account();
        let input_notes = tx_inputs.input_notes();
        let block_hash = tx_inputs.block_header().hash();

        // execute and prove
        let (stack_inputs, advice_inputs) =
            TransactionKernel::prepare_inputs(&tx_inputs, &tx_args, Some(advice_witness));
        let advice_provider: MemAdviceProvider = advice_inputs.into();

        // load the store with account/note/tx_script MASTs
        self.mast_store.load_transaction_code(&tx_inputs, &tx_args);

        let mut host: TransactionHost<_, ()> =
            TransactionHost::new(account.into(), advice_provider, self.mast_store.clone(), None)
                .map_err(TransactionProverError::TransactionHostCreationFailed)?;
        let (stack_outputs, proof) =
            prove(&TransactionKernel::main(), stack_inputs, &mut host, self.proof_options.clone())
                .map_err(TransactionProverError::ProveTransactionProgramFailed)?;

        // extract transaction outputs and process transaction data
        let (advice_provider, account_delta, output_notes, _signatures, _tx_progress) =
            host.into_parts();
        let (_, map, _) = advice_provider.into_parts();
        let tx_outputs =
            TransactionKernel::from_transaction_parts(&stack_outputs, &map.into(), output_notes)
                .map_err(TransactionProverError::InvalidTransactionOutput)?;

        // erase private note information (convert private full notes to just headers)
        let output_notes: Vec<_> = tx_outputs.output_notes.iter().map(OutputNote::shrink).collect();

        let builder = ProvenTransactionBuilder::new(
            account.id(),
            account.init_hash(),
            tx_outputs.account.hash(),
            block_hash,
            proof,
        )
        .add_input_notes(input_notes)
        .add_output_notes(output_notes);

        let builder = match account.is_public() {
            true => {
                let account_update_details = if account.is_new() {
                    let mut account = account.clone();
                    account
                        .apply_delta(&account_delta)
                        .map_err(TransactionProverError::InvalidAccountDelta)?;

                    AccountUpdateDetails::New(account)
                } else {
                    AccountUpdateDetails::Delta(account_delta)
                };

                builder.account_update_details(account_update_details)
            },
            false => builder,
        };

        builder.build().map_err(TransactionProverError::ProvenTransactionError)
    }
}
