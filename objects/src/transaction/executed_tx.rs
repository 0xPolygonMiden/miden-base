use vm_core::StackOutputs;

use super::{TransactionInputs, TransactionScript};
use crate::{
    accounts::validate_account_seed,
    transaction::{utils, Account, AdviceInputs, Digest, InputNotes, Note, StackInputs, Vec, Word},
    ExecutedTransactionError,
};

// EXECUTED TRANSACTION
// ================================================================================================

#[derive(Debug)]
pub struct ExecutedTransaction {
    tx_inputs: TransactionInputs,
    final_account: Account,
    created_notes: Vec<Note>,
    tx_script: Option<TransactionScript>,
}

impl ExecutedTransaction {
    /// Constructs a new [ExecutedTransaction] instance.
    pub fn new(
        tx_inputs: TransactionInputs,
        final_account: Account,
        created_notes: Vec<Note>,
        tx_script: Option<TransactionScript>,
    ) -> Result<Self, ExecutedTransactionError> {
        Self::validate_new_account_seed(&tx_inputs.account, tx_inputs.account_seed)?;
        Ok(Self {
            tx_inputs,
            final_account,
            created_notes,
            tx_script,
        })
    }

    /// Returns the initial account.
    pub fn initial_account(&self) -> &Account {
        &self.tx_inputs.account
    }

    /// Returns the final account.
    pub fn final_account(&self) -> &Account {
        &self.final_account
    }

    /// Returns the consumed notes.
    pub fn input_notes(&self) -> &InputNotes {
        &self.tx_inputs.input_notes
    }

    /// Returns the created notes.
    pub fn output_notes(&self) -> &[Note] {
        &self.created_notes
    }

    /// Returns a reference to the transaction script.
    pub fn tx_script(&self) -> &Option<TransactionScript> {
        &self.tx_script
    }

    /// Returns the block hash.
    pub fn block_hash(&self) -> Digest {
        self.tx_inputs.block_header.hash()
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        utils::generate_stack_inputs(&self.tx_inputs)
    }

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_commitment(&self) -> Digest {
        self.input_notes().commitment()
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(&self.tx_inputs, &self.tx_script)
    }

    /// Returns the stack outputs produced as a result of executing a transaction.
    pub fn stack_outputs(&self) -> StackOutputs {
        utils::generate_stack_outputs(&self.created_notes, &self.final_account.hash())
    }

    /// Returns created notes commitment.
    pub fn created_notes_commitment(&self) -> Digest {
        utils::generate_created_notes_commitment(&self.created_notes)
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------
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
}
