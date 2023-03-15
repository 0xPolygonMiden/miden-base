use super::{Account, Digest, Felt, Hasher, Note, StackInputs, Vec};
use miden_processor::AdviceInputs;

/// A struct that contains all of the data required to execute a transaction. This includes:
/// - account: Account that the transaction is being executed against.
/// - block_ref: The hash of the latest known block.
/// - consumed_notes: A vector of consumed notes.
/// - tx_script_root: An optional transaction script root.
pub struct TransactionInputs {
    account: Account,
    block_ref: Digest,
    consumed_notes: Vec<Note>,
    tx_script_root: Option<Digest>,
}

impl TransactionInputs {
    pub fn new(
        account: Account,
        block_ref: Digest,
        consumed_notes: Vec<Note>,
        tx_script_root: Option<Digest>,
    ) -> Self {
        Self {
            account,
            block_ref,
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

    /// Returns the block reference.
    pub fn block_ref(&self) -> Digest {
        self.block_ref
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
        let mut inputs: Vec<Felt> = Vec::new();
        inputs.extend_from_slice(self.consumed_notes_nullifier_commitment().as_elements());
        inputs.extend_from_slice(self.account.hash().as_elements());
        inputs.push(*self.account.id());
        inputs.extend_from_slice(self.block_ref.as_elements());
        StackInputs::new(inputs)
    }

    /// Returns the advice inputs required when executing the transaction.
    pub fn advice_provider_inputs(&self) -> AdviceInputs {
        let mut inputs: Vec<Felt> = Vec::new();
        let account: [Felt; 16] = (&self.account).into();
        inputs.extend(account);
        inputs.push(Felt::new(self.consumed_notes.len() as u64));
        let note_data: Vec<Felt> = self.consumed_notes.iter().flat_map(<Vec<Felt>>::from).collect();
        inputs.extend(note_data);
        AdviceInputs::default().with_tape(inputs)
    }

    /// Returns the nullifier commitment for all consumed notes.
    pub fn consumed_notes_nullifier_commitment(&self) -> Digest {
        let mut elements: Vec<Felt> = Vec::with_capacity(self.consumed_notes.len() * 8);
        for note in self.consumed_notes.iter() {
            elements.extend_from_slice(note.get_nullifier().as_elements());
            elements.extend_from_slice(note.script().hash().as_elements());
        }
        Hasher::hash_elements(&elements)
    }
}
