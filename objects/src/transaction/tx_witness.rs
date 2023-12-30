use super::{AccountId, AdviceInputs, Digest, Felt, Program};

// TRANSACTION WITNESS
// ================================================================================================

/// A [TransactionWitness] is the minimum required data required to execute and prove a Miden rollup
/// transaction.
///
/// The [TransactionWitness] is composed of:
/// - account_id: the account id of the account the transaction is being executed against.
/// - initial_account_hash: the hash of the initial state of the account the transaction is being
///   executed against.
/// - block_hash: the block hash of the latest known block.
/// - input_notes_hash: a commitment to the consumed notes of the transaction.
/// - tx_script_root: an optional transaction script root.
/// - program: the transaction [Program]
/// - advice_witness: the advice inputs for the transaction
pub struct TransactionWitness {
    account_id: AccountId,
    initial_account_hash: Digest,
    block_hash: Digest,
    input_notes_hash: Digest,
    tx_script_root: Option<Digest>,
    program: Program,
    advice_witness: AdviceInputs,
}

impl TransactionWitness {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionWitness] from the provided data.
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        block_hash: Digest,
        input_notes_hash: Digest,
        tx_script_root: Option<Digest>,
        program: Program,
        advice_witness: AdviceInputs,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            block_hash,
            input_notes_hash,
            tx_script_root,
            program,
            advice_witness,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the account id of the account the transaction is executed against.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the initial account hash of the account the transaction is executed against.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }
    /// Returns a commitment to the notes consumed by the transaction.
    pub fn input_notes_hash(&self) -> Digest {
        self.input_notes_hash
    }

    /// Returns the block hash of the latest known block.
    pub fn block_hash(&self) -> Digest {
        self.block_hash
    }

    /// Returns the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction [Program].
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the advice inputs for the transaction.
    pub fn advice_inputs(&self) -> &AdviceInputs {
        &self.advice_witness
    }

    // ADVICE DATA EXTRACTORS
    // --------------------------------------------------------------------------------------------

    /// Returns data from the advice map located under `self.input_notes_hash` key.
    pub fn input_note_data(&self) -> Option<&[Felt]> {
        // TODO: return None if input_notes_hash == EMPTY_WORD?
        self.advice_witness.mapped_values(&self.input_notes_hash.as_bytes())
    }
}
