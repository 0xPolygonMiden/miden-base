use super::{
    utils, Account, AdviceInputs, BlockHeader, ChainMmr, Digest, InputNotes,
    PreparedTransactionError, Program, StackInputs, TransactionInputs, TransactionScript, Word,
};
use crate::accounts::validate_account_seed;

// PREPARED TRANSACTION
// ================================================================================================

/// A struct that contains all of the data required to execute a transaction.
///
/// This includes:
/// - A an executable program which defines the transaction.
/// - An optional transaction script.
/// - A set of inputs against which the transaction program should be executed.
#[derive(Debug)]
pub struct PreparedTransaction {
    program: Program,
    tx_script: Option<TransactionScript>,
    tx_inputs: TransactionInputs,
}

impl PreparedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [PreparedTransaction] instantiated from the provided executable transaction
    /// program and inputs required to execute this program.
    pub fn new(
        program: Program,
        tx_script: Option<TransactionScript>,
        tx_inputs: TransactionInputs,
    ) -> Result<Self, PreparedTransactionError> {
        validate_new_account_seed(&tx_inputs.account, tx_inputs.account_seed)?;
        Ok(Self { program, tx_script, tx_inputs })
    }

    // ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the transaction program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the account.
    pub fn account(&self) -> &Account {
        &self.tx_inputs.account
    }

    /// Returns the block header.
    pub fn block_header(&self) -> &BlockHeader {
        &self.tx_inputs.block_header
    }

    /// Returns the block chain.
    pub fn block_chain(&self) -> &ChainMmr {
        &self.tx_inputs.block_chain
    }

    /// Returns the input notes.
    pub fn input_notes(&self) -> &InputNotes {
        &self.tx_inputs.input_notes
    }

    /// Return a reference the transaction script.
    pub fn tx_script(&self) -> &Option<TransactionScript> {
        &self.tx_script
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        let initial_acct_hash = if self.account().is_new() {
            Digest::default()
        } else {
            self.account().hash()
        };
        utils::generate_stack_inputs(
            &self.account().id(),
            initial_acct_hash,
            self.input_notes().commitment(),
            self.block_header(),
        )
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(
            self.account(),
            self.tx_inputs.account_seed,
            self.block_header(),
            self.block_chain(),
            self.input_notes(),
            &self.tx_script,
        )
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------

    /// Consumes the prepared transaction and returns its parts.
    pub fn into_parts(
        self,
    ) -> (Account, BlockHeader, ChainMmr, InputNotes, Program, Option<TransactionScript>) {
        (
            self.tx_inputs.account,
            self.tx_inputs.block_header,
            self.tx_inputs.block_chain,
            self.tx_inputs.input_notes,
            self.program,
            self.tx_script,
        )
    }
}

// HELPER FUNCTIONS
// ================================================================================================

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
