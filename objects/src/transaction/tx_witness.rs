use super::{
    AccountId, AdviceInputs, Digest, Felt, Hasher, Nullifier, Program, StackInputs, StarkField,
    TransactionWitnessError, Vec, Word, WORD_SIZE,
};

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
    pub fn block_hash(&self) -> &Digest {
        &self.block_hash
    }

    /// Returns a vector of [Nullifier] for all consumed notes in the transaction.
    ///
    /// # Errors
    /// - If the consumed notes data is not found in the advice map.
    /// - If the consumed notes data is not well formed.
    pub fn input_notes_info(&self) -> Result<Vec<Nullifier>, TransactionWitnessError> {
        // fetch input notes data from the advice map
        let notes_data = self
            .advice_witness
            .mapped_values(&self.input_notes_hash.as_bytes())
            .ok_or(TransactionWitnessError::ConsumedNoteDataNotFound)?;

        // extract the notes from the first fetch and instantiate a vector to hold nullifiers
        let num_notes = notes_data[0].as_int();
        let mut input_notes_info = Vec::with_capacity(num_notes as usize);

        // iterate over the notes and extract the nullifier and script root
        let mut note_ptr = 1;
        while note_ptr < notes_data.len() {
            // make sure there is enough data to read (note data is well formed)
            if note_ptr + 5 * WORD_SIZE > notes_data.len() {
                return Err(TransactionWitnessError::InvalidConsumedNoteDataLength);
            }

            // compute the nullifier and extract script root and number of assets
            let (nullifier, num_assets) = extract_note_data(&notes_data[note_ptr..]);

            // push the [ConsumedNoteInfo] to the vector
            input_notes_info.push(nullifier.into());

            // round up the number of assets to the next multiple of 2 to account for asset padding
            let num_assets = (num_assets + 1) & !1;

            // increment note pointer
            note_ptr += (num_assets as usize * WORD_SIZE) + 30;
        }

        debug_assert_eq!(
            self.input_notes_hash,
            Hasher::hash_elements(
                &input_notes_info
                    .iter()
                    .flat_map(|info| {
                        let mut elements = Word::from(info).to_vec();
                        elements.extend_from_slice(&Word::default());
                        elements
                    })
                    .collect::<Vec<_>>()
            )
        );

        Ok(input_notes_info)
    }

    /// Returns the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction [Program].
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the stack inputs for the transaction.
    pub fn get_stack_inputs(&self) -> StackInputs {
        let mut inputs: Vec<Felt> = Vec::with_capacity(13);
        inputs.extend(*self.input_notes_hash);
        inputs.extend(*self.initial_account_hash);
        inputs.push(self.account_id.into());
        inputs.extend(*self.block_hash);
        StackInputs::new(inputs)
    }

    /// Returns the advice inputs for the transaction.
    pub fn advice_inputs(&self) -> &AdviceInputs {
        &self.advice_witness
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------
    /// Consumes the witness and returns its parts.
    pub fn into_parts(
        self,
    ) -> (AccountId, Digest, Digest, Digest, Option<Digest>, Program, AdviceInputs) {
        (
            self.account_id,
            self.initial_account_hash,
            self.block_hash,
            self.input_notes_hash,
            self.tx_script_root,
            self.program,
            self.advice_witness,
        )
    }
}

// HELPERS
// ================================================================================================
/// Extracts and returns the nullifier and the number of assets from the provided note data.
///
/// Expects the note data to be organized as follows:
/// [CN_SN, CN_SR, CN_IR, CN_VR, CN_M]
///
/// - CN_SN is the serial number of the consumed note.
/// - CN_SR is the script root of the consumed note.
/// - CN_IR is the inputs root of the consumed note.
/// - CN_VR is the vault root of the consumed note.
/// - CN1_M is the metadata of the consumed note.
fn extract_note_data(note_data: &[Felt]) -> (Digest, u64) {
    // compute the nullifier
    let nullifier = Hasher::hash_elements(&note_data[..4 * WORD_SIZE]);

    // extract the number of assets
    let num_assets = note_data[4 * WORD_SIZE].as_int();

    (nullifier, num_assets)
}
