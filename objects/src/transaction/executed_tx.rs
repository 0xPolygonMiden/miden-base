use crate::{
    accounts::validate_account_seed,
    transaction::{
        utils, Account, AdviceInputs, BlockHeader, ChainMmr, ConsumedNotes, Digest, Note,
        StackInputs, Vec, Word,
    },
    ExecutedTransactionError,
};
use vm_core::StackOutputs;

#[derive(Debug)]
pub struct ExecutedTransaction {
    initial_account: Account,
    initial_account_seed: Option<Word>,
    final_account: Account,
    consumed_notes: ConsumedNotes,
    created_notes: Vec<Note>,
    tx_script_root: Option<Digest>,
    block_header: BlockHeader,
    block_chain: ChainMmr,
}

impl ExecutedTransaction {
    /// Constructs a new [ExecutedTransaction] instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        initial_account: Account,
        initial_account_seed: Option<Word>,
        final_account: Account,
        consumed_notes: Vec<Note>,
        created_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
        block_header: BlockHeader,
        block_chain: ChainMmr,
    ) -> Result<Self, ExecutedTransactionError> {
        Self::validate_new_account_seed(&initial_account, initial_account_seed)?;
        Ok(Self {
            initial_account,
            initial_account_seed,
            final_account,
            consumed_notes: ConsumedNotes::new(consumed_notes),
            created_notes,
            tx_script_root,
            block_header,
            block_chain,
        })
    }

    /// Returns the initial account.
    pub fn initial_account(&self) -> &Account {
        &self.initial_account
    }

    /// Returns the final account.
    pub fn final_account(&self) -> &Account {
        &self.final_account
    }

    /// Returns the consumed notes.
    pub fn consumed_notes(&self) -> &ConsumedNotes {
        &self.consumed_notes
    }

    /// Returns the created notes.
    pub fn created_notes(&self) -> &[Note] {
        &self.created_notes
    }

    /// Returns the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the block reference.
    pub fn block_hash(&self) -> Digest {
        self.block_header.hash()
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        let initial_acct_hash = if self.initial_account.is_new() {
            Digest::default()
        } else {
            self.initial_account.hash()
        };
        utils::generate_stack_inputs(
            &self.initial_account.id(),
            initial_acct_hash,
            self.consumed_notes.commitment(),
            &self.block_header,
        )
    }

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_commitment(&self) -> Digest {
        self.consumed_notes.commitment()
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(
            &self.initial_account,
            self.initial_account_seed,
            &self.block_header,
            &self.block_chain,
            &self.consumed_notes,
        )
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
