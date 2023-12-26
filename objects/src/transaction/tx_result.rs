use vm_processor::{AdviceInputs, Program};

use crate::{
    accounts::{AccountDelta, AccountId},
    transaction::{
        InputNotes, OutputNotes, TransactionInputs, TransactionOutputs, TransactionWitness,
    },
    Digest, TransactionResultError,
};

// TRANSACTION RESULT
// ================================================================================================

/// [TransactionResult] represents the result of the execution of the transaction kernel.
///
/// [TransactionResult] is a container for the following data:
/// - account_id: the ID of the account against which the transaction was executed.
/// - initial_account_hash: the initial account hash.
/// - final_account_hash: the final account hash.
/// - account_delta: a delta between the initial and final accounts.
/// - consumed_notes: the notes consumed by the transaction.
/// - created_notes: the notes created by the transaction.
/// - block_hash: the hash of the block against which the transaction was executed.
/// - program: the program that was executed.
/// - tx_script_root: the script root of the transaction.
/// - advice_witness: an advice witness that contains the minimum required data to execute a tx.
#[derive(Debug, Clone)]
pub struct TransactionResult {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    account_delta: AccountDelta,
    input_notes: InputNotes,
    output_notes: OutputNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_witness: AdviceInputs,
}

impl TransactionResult {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
    pub fn new(
        tx_inputs: TransactionInputs,
        tx_outputs: TransactionOutputs,
        account_delta: AccountDelta,
        program: Program,
        tx_script_root: Option<Digest>,
        advice_witness: AdviceInputs,
    ) -> Result<Self, TransactionResultError> {
        Ok(Self {
            account_id: tx_inputs.account.id(),
            initial_account_hash: tx_inputs.account.hash(),
            final_account_hash: tx_outputs.account.hash(),
            account_delta,
            input_notes: tx_inputs.input_notes,
            output_notes: tx_outputs.output_notes,
            block_hash: tx_inputs.block_header.hash(),
            program,
            tx_script_root,
            advice_witness,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the account for which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns a reference to the initial account hash.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }

    /// Returns a reference to the final account hash.
    pub fn final_account_hash(&self) -> Digest {
        self.final_account_hash
    }

    /// Returns a reference to the account delta.
    pub fn account_delta(&self) -> &AccountDelta {
        &self.account_delta
    }

    /// Returns a reference to the consumed notes.
    pub fn input_notes(&self) -> &InputNotes {
        &self.input_notes
    }

    /// Returns a reference to the created notes.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
    }

    /// Returns the block hash the transaction was executed against.
    pub fn block_hash(&self) -> Digest {
        self.block_hash
    }

    /// Returns a reference the transaction program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the root of the transaction script.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns a reference to the advice provider.
    pub fn advice_witness(&self) -> &AdviceInputs {
        &self.advice_witness
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------
    pub fn into_witness(self) -> TransactionWitness {
        TransactionWitness::new(
            self.account_id,
            self.initial_account_hash,
            self.block_hash,
            self.input_notes.commitment(),
            self.tx_script_root,
            self.program,
            self.advice_witness,
        )
    }
}
