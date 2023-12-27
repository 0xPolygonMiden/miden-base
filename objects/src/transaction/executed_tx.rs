use vm_core::StackInputs;

use super::{
    utils, AdviceInputs, InputNotes, OutputNotes, TransactionInputs, TransactionOutputs,
    TransactionScript,
};
use crate::{
    accounts::{validate_account_seed, Account, AccountStub},
    BlockHeader, ExecutedTransactionError, Word,
};

// EXECUTED TRANSACTION
// ================================================================================================

#[derive(Debug)]
pub struct ExecutedTransaction {
    tx_inputs: TransactionInputs,
    tx_outputs: TransactionOutputs,
    tx_script: Option<TransactionScript>,
}

impl ExecutedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Constructs a new [ExecutedTransaction] instance.
    pub fn new(
        tx_inputs: TransactionInputs,
        tx_outputs: TransactionOutputs,
        tx_script: Option<TransactionScript>,
    ) -> Result<Self, ExecutedTransactionError> {
        validate_new_account_seed(&tx_inputs.account, tx_inputs.account_seed)?;
        Ok(Self { tx_inputs, tx_outputs, tx_script })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the initial account.
    pub fn initial_account(&self) -> &Account {
        &self.tx_inputs.account
    }

    /// Returns the final account.
    pub fn final_account(&self) -> &AccountStub {
        &self.tx_outputs.account
    }

    /// Returns the consumed notes.
    pub fn input_notes(&self) -> &InputNotes {
        &self.tx_inputs.input_notes
    }

    /// Returns the created notes.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.tx_outputs.output_notes
    }

    /// Returns a reference to the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }

    /// Returns the block header.
    pub fn block_header(&self) -> &BlockHeader {
        &self.tx_inputs.block_header
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        utils::generate_stack_inputs(&self.tx_inputs)
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(&self.tx_inputs, &self.tx_script)
    }
}

// HELPER FUNCTIONS
// ================================================================================================

/// Validates that a valid account seed has been provided if the account the transaction is
/// being executed against is new.
fn validate_new_account_seed(
    account: &Account,
    seed: Option<Word>,
) -> Result<(), ExecutedTransactionError> {
    match (account.is_new(), seed) {
        (true, Some(seed)) => validate_account_seed(account, seed)
            .map_err(ExecutedTransactionError::InvalidAccountIdSeedError),
        (true, None) => Err(ExecutedTransactionError::AccountIdSeedNoteProvided),
        _ => Ok(()),
    }
}
