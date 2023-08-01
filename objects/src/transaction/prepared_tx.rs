use super::{
    utils, Account, AdviceInputs, BlockHeader, ChainMmr, ConsumedNotes, Digest, Note,
    PreparedTransactionError, Program, StackInputs, Vec, Word,
};
use crate::validate_account_seed;

/// A struct that contains all of the data required to execute a transaction. This includes:
/// - account: Account that the transaction is being executed against.
/// - account_seed: An optional account seed used to create a new account.
/// - block_header: The header of the latest known block.
/// - block_chain: The chain mmr associated with the latest known block.
/// - consumed_notes: A vector of consumed notes.
/// - tx_script_root: An optional transaction script root.
/// - tx_program: The transaction program.
#[derive(Debug)]
pub struct PreparedTransaction {
    account: Account,
    account_seed: Option<Word>,
    block_header: BlockHeader,
    block_chain: ChainMmr,
    consumed_notes: ConsumedNotes,
    tx_script_root: Option<Digest>,
    tx_program: Program,
}

impl PreparedTransaction {
    pub fn new(
        account: Account,
        account_seed: Option<Word>,
        block_header: BlockHeader,
        block_chain: ChainMmr,
        consumed_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
        tx_program: Program,
    ) -> Result<Self, PreparedTransactionError> {
        Self::validate_new_account_seed(&account, account_seed)?;
        Ok(Self {
            account,
            account_seed,
            block_header,
            block_chain,
            consumed_notes: ConsumedNotes::new(consumed_notes),
            tx_script_root,
            tx_program,
        })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the account.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// Returns the block header.
    pub fn block_header(&self) -> &BlockHeader {
        &self.block_header
    }

    /// Returns the block chain.
    pub fn block_chain(&self) -> &ChainMmr {
        &self.block_chain
    }

    /// Returns the consumed notes.
    pub fn consumed_notes(&self) -> &ConsumedNotes {
        &self.consumed_notes
    }

    /// Return the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the transaction program.
    pub fn tx_program(&self) -> &Program {
        &self.tx_program
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        let initial_acct_hash = if self.account.is_new() {
            Digest::default()
        } else {
            self.account.hash()
        };
        utils::generate_stack_inputs(
            &self.account.id(),
            initial_acct_hash,
            self.consumed_notes.commitment(),
            &self.block_header,
        )
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(
            &self.account,
            self.account_seed,
            &self.block_header,
            &self.block_chain,
            &self.consumed_notes,
        )
    }

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_commitment(&self) -> Digest {
        self.consumed_notes.commitment()
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
    /// Validates that a valid account seed has been provided if the account the transaction is
    /// being executed against is new.
    fn validate_new_account_seed(
        account: &Account,
        seed: Option<Word>,
    ) -> Result<(), PreparedTransactionError> {
        match (account.is_new(), seed) {
            (true, Some(seed)) => validate_account_seed(account, seed)
                .map_err(PreparedTransactionError::InvalidAccountIdSeedError),
            (true, None) => Err(PreparedTransactionError::AccountIdSeedNoteProvided),
            _ => Ok(()),
        }
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------
    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(
        self,
    ) -> (Account, BlockHeader, ChainMmr, ConsumedNotes, Program, Option<Digest>) {
        (
            self.account,
            self.block_header,
            self.block_chain,
            self.consumed_notes,
            self.tx_program,
            self.tx_script_root,
        )
    }
}
