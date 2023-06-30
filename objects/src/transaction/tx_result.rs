use super::{
    AccountDelta, AccountId, AccountStub, AdviceProvider, ConsumedNotes, CreatedNotes, Digest,
    Program, StackOutputs, TransactionResultError, TransactionWitness, TryFromVmResult,
};
use crypto::utils::collections::Diff;
use miden_processor::RecAdviceProvider;

/// [TransactionResult] represents the result of the execution of the transaction kernel.
///
/// [TransactionResult] is a container for the following data:
/// - initial_account_stub: a stub of the account before the transaction was executed.
/// - final_account_stub: a stub of the account after the transaction was executed.
/// - account_delta: a delta between the initial and final account stubs.
/// - consumed_notes: the notes consumed by the transaction.
/// - created_notes: the notes created by the transaction.
/// - block_hash: the hash of the block against which the transaction was executed.
/// - program: the program that was executed.
/// - tx_script_root: the script root of the transaction.
/// - advice_provider: the final state of the advice provider.
/// - stack_outputs: the stack outputs produced by the transaction.
#[derive(Debug, Clone)]
pub struct TransactionResult<T: AdviceProvider> {
    initial_account_stub: AccountStub,
    final_account_stub: AccountStub,
    account_delta: AccountDelta,
    consumed_notes: ConsumedNotes,
    created_notes: CreatedNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_provider: T,
    stack_outputs: StackOutputs,
}

impl<T: AdviceProvider> TransactionResult<T> {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
    pub fn new(
        initial_account_stub: AccountStub,
        consumed_notes: ConsumedNotes,
        block_hash: Digest,
        program: Program,
        tx_script_root: Option<Digest>,
        advice_provider: T,
        stack_outputs: StackOutputs,
    ) -> Result<Self, TransactionResultError> {
        let TransactionOutputs {
            final_account_stub,
            created_notes,
        } = TransactionOutputs::try_from_vm_result(&stack_outputs, &advice_provider)?;

        let account_delta = initial_account_stub.diff(&final_account_stub);

        Ok(Self {
            initial_account_stub,
            final_account_stub,
            account_delta,
            consumed_notes,
            created_notes,
            block_hash,
            program,
            tx_script_root,
            advice_provider,
            stack_outputs,
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the account for which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.initial_account_stub.id()
    }

    /// Returns a reference to the initial account stub.
    pub fn initial_account_stub(&self) -> &AccountStub {
        &self.initial_account_stub
    }

    /// Returns a reference to the final account stub.
    pub fn final_account_stub(&self) -> &AccountStub {
        &self.final_account_stub
    }

    /// Returns a reference to the account delta.
    pub fn account_delta(&self) -> &AccountDelta {
        &self.account_delta
    }

    /// Returns a reference to the consumed notes.
    pub fn consumed_notes(&self) -> &ConsumedNotes {
        &self.consumed_notes
    }

    /// Returns a reference to the created notes.
    pub fn created_notes(&self) -> &CreatedNotes {
        &self.created_notes
    }

    /// Returns the block hash the transaction was executed against.
    pub fn block_hash(&self) -> Digest {
        self.block_hash
    }

    /// Returns a reference the transaction program.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the root of the transaction script.
    pub fn tx_script_root(&self) -> Option<Digest> {
        self.tx_script_root
    }

    /// Returns a reference to the advice provider.
    pub fn advice_provider(&self) -> &T {
        &self.advice_provider
    }

    /// Returns a reference to the stack outputs.
    pub fn stack_outputs(&self) -> &StackOutputs {
        &self.stack_outputs
    }
}

impl TransactionResult<RecAdviceProvider> {
    // CONSUMERS
    // --------------------------------------------------------------------------------------------
    pub fn into_witness(self) -> TransactionWitness {
        TransactionWitness::new(
            self.initial_account_stub.id(),
            self.initial_account_stub.hash(),
            self.block_hash,
            self.consumed_notes.commitment(),
            self.tx_script_root,
            self.program,
            self.advice_provider.into_proof(),
        )
    }
}

// TRANSACTION OUTPUTS
// ================================================================================================
/// [TransactionOutputs] stores the outputs produced from transaction execution.
///
/// [TransactionOutputs] is a container for the following data:
/// - final_account_stub: a stub of the account after the transaction was executed.
/// - created_notes: the notes created by the transaction.
pub struct TransactionOutputs {
    pub final_account_stub: AccountStub,
    pub created_notes: CreatedNotes,
}

impl<T: AdviceProvider> TryFromVmResult<T> for TransactionOutputs {
    type Error = TransactionResultError;

    /// Tries to create [TransactionOutputs] from the provided stack outputs and advice provider.
    fn try_from_vm_result(
        stack_outputs: &miden_core::StackOutputs,
        advice_provider: &T,
    ) -> Result<Self, Self::Error> {
        let final_account_stub = AccountStub::try_from_vm_result(stack_outputs, advice_provider)?;
        let created_notes = CreatedNotes::try_from_vm_result(stack_outputs, advice_provider)?;
        Ok(Self {
            final_account_stub,
            created_notes,
        })
    }
}
