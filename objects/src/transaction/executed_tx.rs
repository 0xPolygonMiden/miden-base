use super::{utils, Account, AdviceInputs, BlockHeader, ChainMmr, Digest, Note, StackInputs};
use miden_core::StackOutputs;

pub struct ExecutedTransaction {
    initial_account: Account,
    final_account: Account,
    consumed_notes: Vec<Note>,
    created_notes: Vec<Note>,
    tx_script_root: Option<Digest>,
    block_header: BlockHeader,
    block_chain: ChainMmr,
}

impl ExecutedTransaction {
    pub fn new(
        initial_account: Account,
        final_account: Account,
        consumed_notes: Vec<Note>,
        created_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
        block_header: BlockHeader,
        block_chain: ChainMmr,
    ) -> Self {
        Self {
            initial_account,
            final_account,
            consumed_notes,
            created_notes,
            tx_script_root,
            block_header,
            block_chain,
        }
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
    pub fn consumed_notes(&self) -> &[Note] {
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
        utils::generate_stack_inputs(
            &self.initial_account.id(),
            &self.initial_account.hash(),
            &self.consumed_notes,
            &self.block_header,
        )
    }

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_commitment(&self) -> Digest {
        utils::generate_consumed_notes_commitment(&self.consumed_notes)
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(
            &self.initial_account,
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
}
