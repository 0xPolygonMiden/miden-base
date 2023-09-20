use crate::{
    accounts::{Account, AccountDelta, AccountId},
    transaction::{ConsumedNotes, CreatedNotes, FinalAccountStub, TransactionWitness},
    TransactionResultError,
};
use crypto::hash::rpo::RpoDigest as Digest;
use vm_processor::{AdviceInputs, Program};

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
    consumed_notes: ConsumedNotes,
    created_notes: CreatedNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_witness: AdviceInputs,
}

impl TransactionResult {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        initial_account: Account,
        final_account_stub: FinalAccountStub,
        account_delta: AccountDelta,
        consumed_notes: ConsumedNotes,
        created_notes: CreatedNotes,
        block_hash: Digest,
        program: Program,
        tx_script_root: Option<Digest>,
        advice_witness: AdviceInputs,
    ) -> Result<Self, TransactionResultError> {
        Ok(Self {
            account_id: initial_account.id(),
            initial_account_hash: initial_account.hash(),
            final_account_hash: final_account_stub.0.hash(),
            account_delta,
            consumed_notes,
            created_notes,
            block_hash,
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
    pub fn consumed_notes(&self) -> &ConsumedNotes {
        &self.consumed_notes
    }

    /// Returns a reference to the created notes.
    pub fn created_notes(&self) -> &CreatedNotes {
        &self.created_notes
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
            self.consumed_notes.commitment(),
            self.tx_script_root,
            self.program,
            self.advice_witness,
        )
    }
}
