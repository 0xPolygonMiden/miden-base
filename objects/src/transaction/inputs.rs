use super::{utils, Account, AdviceInputs, BlockHeader, ChainMmr, Digest, Note, StackInputs, Vec};

/// A struct that contains all of the data required to execute a transaction. This includes:
/// - account: Account that the transaction is being executed against.
/// - block_header: The header of the latest known block.
/// - block_chain: The chain mmr associated with the latest known blcok.
/// - consumed_notes: A vector of consumed notes.
/// - tx_script_root: An optional transaction script root.
pub struct TransactionInputs {
    account: Account,
    block_header: BlockHeader,
    block_chain: ChainMmr,
    consumed_notes: Vec<Note>,
    tx_script_root: Option<Digest>,
}

impl TransactionInputs {
    pub fn new(
        account: Account,
        block_header: BlockHeader,
        block_chain: ChainMmr,
        consumed_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
    ) -> Self {
        Self {
            account,
            block_header,
            block_chain,
            consumed_notes,
            tx_script_root,
        }
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
    pub fn consumed_notes(&self) -> &[Note] {
        &self.consumed_notes
    }

    /// Return the transaction script root.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns the stack inputs required when executing the transaction.
    pub fn stack_inputs(&self) -> StackInputs {
        utils::generate_stack_inputs(
            &self.account.id(),
            &self.account.hash(),
            &self.consumed_notes,
            &self.block_header,
        )
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        utils::generate_advice_provider_inputs(
            &self.account,
            &self.block_header,
            &self.block_chain,
            &self.consumed_notes,
        )
    }

    /// Returns the consumed notes commitment.
    pub fn consumed_notes_commitment(&self) -> Digest {
        utils::generate_consumed_notes_commitment(&self.consumed_notes)
    }
}
