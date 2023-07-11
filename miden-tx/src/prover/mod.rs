use super::{
    DataStore, MemAdviceProvider, ProgramAst, ProvenTransaction, TransactionExecutor,
    TransactionProverError, TransactionWitness,
};
use miden_objects::{
    notes::NoteOrigin, transaction::TransactionOutputs, AccountId, TryFromVmResult,
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

    pub fn prove_with_transaction_executor<D: DataStore>(
        &self,
        account_id: AccountId,
        block_ref: u32,
        note_origins: &[NoteOrigin],
        tx_script: Option<ProgramAst>,
        tx_executor: &mut TransactionExecutor<D>,
    ) -> Result<ProvenTransaction, TransactionProverError> {
        // prepare transaction
        let transaction = tx_executor
            .prepare_transaction(account_id, block_ref, note_origins, tx_script)
            .unwrap();
        let mut advice_provider: MemAdviceProvider = transaction.advice_provider_inputs().into();

        // prove transaction program
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
            account_id,
            transaction.account().hash(),
            transaction_outputs.final_account_stub.0.hash(),
            consumed_notes_info,
            created_notes_info,
            transaction.tx_script_root(),
            transaction.block_header().hash(),
            proof,
        ))
    }

    pub fn prove_transaction(
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
            block_hash,
            proof,
        ))
    }
}
