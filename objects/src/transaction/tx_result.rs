use super::{
    Account, AccountDelta, AccountError, AccountId, AccountStub, AdviceProvider, ConsumedNotes,
    CreatedNotes, Digest, Felt, Program, StackOutputs, TransactionResultError, TransactionWitness,
    TryFromVmResult, Word, WORD_SIZE,
};
use crate::{AccountCode, AccountStorage};
use assembly::{ast::ModuleAst, Assembler};
use crypto::merkle::{SimpleSmt, SimpleSmtConfig, TryFromMerkleStore};
use miden_lib::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_IDX, ACCT_NONCE_IDX,
    ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};
use miden_processor::{AdviceInputs, RecAdviceProvider};
use miden_test_utils::collections::Diff;

/// [TransactionResult] represents the result of the execution of the transaction kernel.
///
/// [TransactionResult] is a container for the following data:
/// - account_id: the ID of the account against which the transaction was executed.
/// - initial_account_hash: the initial account hash.
/// - final_account_hash: the final account hash.
/// - account_delta: a delta between the initial and final accounts.
/// - consumed_notes: the notes consumed by the transaction.
/// - created_notes: the notes created by the transaction.
/// - block_hash: the hash of the block against which the transaction was executed.
/// - program: the program that was executed.
/// - tx_script_root: the script root of the transaction.
/// - advice_witness: an advice witness that contains the minimum required data to execute a tx.
#[derive(Debug, Clone)]
pub struct TransactionResult {
    account_id: AccountId,
    initial_account_hash: Digest,
    final_account_hash: Digest,
    account_delta: AccountDelta,
    consumed_notes: ConsumedNotes,
    created_notes: CreatedNotes,
    block_hash: Digest,
    program: Program,
    tx_script_root: Option<Digest>,
    advice_witness: AdviceInputs,
}

impl TransactionResult {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    /// Creates a new [TransactionResult] from the provided data, advice provider and stack outputs.
    pub fn new(
        initial_account: Account,
        consumed_notes: ConsumedNotes,
        block_hash: Digest,
        program: Program,
        tx_script_root: Option<Digest>,
        advice_provider: RecAdviceProvider,
        stack_outputs: StackOutputs,
        acct_code_update: Option<ModuleAst>,
    ) -> Result<Self, TransactionResultError> {
        // TODO:
        let TransactionOutputs {
            final_account_stub,
            created_notes,
        } = TransactionOutputs::try_from_vm_result(&stack_outputs, &advice_provider)?;

        let storage_delta = initial_account.storage().diff(&final_account_stub.0.storage());
        let nonce_delta = if initial_account.nonce() != final_account_stub.0.nonce() {
            Some(final_account_stub.0.nonce())
        } else {
            None
        };

        // TODO: Currently we mock these diffs by comparing with self. We should change this to compare
        // with final account once we can extract code and vault data from the VM. We made need a
        // separate pathway for the code delta. This can potentially be provided by the client.
        let vault_delta = initial_account.vault().diff(&initial_account.vault());

        // assert the updated module matches the code root in the final account stub
        if let Some(delta) = acct_code_update.as_ref() {
            let updated_acct_code =
                AccountCode::new(AccountId::default(), delta.clone(), &mut Assembler::default())
                    .map_err(TransactionResultError::UpdatedAccountCodeInvalid)?;
            if updated_acct_code.root() != final_account_stub.0.code_root() {
                return Err(TransactionResultError::InconsistentAccountCodeHash(
                    updated_acct_code.root(),
                    final_account_stub.0.code_root(),
                ));
            }
        }

        let account_delta = AccountDelta {
            code: acct_code_update,
            nonce: nonce_delta,
            storage: storage_delta,
            vault: vault_delta,
        };

        Ok(Self {
            account_id: initial_account.id(),
            initial_account_hash: initial_account.hash(),
            final_account_hash: final_account_stub.0.hash(),
            account_delta,
            consumed_notes,
            created_notes,
            block_hash,
            program,
            tx_script_root,
            advice_witness: advice_provider.into_proof(),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the ID of the account for which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns a reference to the initial account hash.
    pub fn initial_account_hash(&self) -> Digest {
        self.initial_account_hash
    }

    /// Returns a reference to the final account hash.
    pub fn final_account_hash(&self) -> Digest {
        self.final_account_hash
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
    pub fn advice_witness(&self) -> &AdviceInputs {
        &self.advice_witness
    }

    // CONSUMERS
    // --------------------------------------------------------------------------------------------
    pub fn into_witness(self) -> TransactionWitness {
        TransactionWitness::new(
            self.account_id,
            self.initial_account_hash,
            self.block_hash,
            self.consumed_notes.commitment(),
            self.tx_script_root,
            self.program,
            self.advice_witness,
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
    pub final_account_stub: FinalAccountStub,
    pub created_notes: CreatedNotes,
}

impl<T: AdviceProvider> TryFromVmResult<T> for TransactionOutputs {
    type Error = TransactionResultError;

    /// Tries to create [TransactionOutputs] from the provided stack outputs and advice provider.
    fn try_from_vm_result(
        stack_outputs: &miden_core::StackOutputs,
        advice_provider: &T,
    ) -> Result<Self, Self::Error> {
        let final_account_stub =
            FinalAccountStub::try_from_vm_result(stack_outputs, advice_provider)?;
        let created_notes = CreatedNotes::try_from_vm_result(stack_outputs, advice_provider)?;
        Ok(Self {
            final_account_stub,
            created_notes,
        })
    }
}

// FINAL ACCOUNT STUB
// ================================================================================================
pub struct FinalAccountStub(pub AccountStub);

impl<T: AdviceProvider> TryFromVmResult<T> for FinalAccountStub {
    type Error = TransactionResultError;

    fn try_from_vm_result(result: &StackOutputs, advice_provider: &T) -> Result<Self, Self::Error> {
        const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 1;

        let final_account_hash: Word = result.stack()[FINAL_ACCOUNT_HASH_WORD_IDX * WORD_SIZE
            ..(FINAL_ACCOUNT_HASH_WORD_IDX + 1) * WORD_SIZE]
            .iter()
            .rev()
            .map(|x| Felt::from(*x))
            .collect::<Vec<_>>()
            .try_into()
            .expect("word size is correct");
        let final_account_hash: Digest = final_account_hash.into();

        // extract final account data from the advice map
        let final_account_data = advice_provider
            .get_mapped_values(&final_account_hash.as_bytes())
            .ok_or(TransactionResultError::FinalAccountDataNotFound)?;
        let (id, vault_root, storage_root, code_root, nonce) =
            parse_stub_account_commitments(final_account_data)
                .map_err(TransactionResultError::FinalAccountStubDataInvalid)?;

        // extract account storage
        let account_storage = extract_account_storage(storage_root, advice_provider)?;

        Ok(Self(AccountStub::new(id, nonce, vault_root, account_storage, code_root)))
    }
}

/// Parses the [AccountStorage] associated with the provided account storage root using data from
/// the advice provider.
fn extract_account_storage<T: AdviceProvider>(
    storage_root: Digest,
    advice_provider: &T,
) -> Result<AccountStorage, TransactionResultError> {
    let storage_slots_config = SimpleSmtConfig {
        root: storage_root,
        depth: AccountStorage::STORAGE_TREE_DEPTH,
    };
    let storage_slots_data = advice_provider.get_store_subset([storage_root].iter());
    let storage_slots = SimpleSmt::try_from_merkle_store(storage_slots_config, &storage_slots_data)
        .map_err(TransactionResultError::ExtractAccountStorageSlotsFailed)?;
    let storage_store = advice_provider
        .get_store_subset(storage_slots.leaves().map(|(_k, v)| Into::<Digest>::into(*v)));
    let account_storage = AccountStorage::from_parts(storage_slots, storage_store);
    Ok(account_storage)
}

/// Parses the stub account data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage root, code root, and nonce.
fn parse_stub_account_commitments(
    elements: &[Felt],
) -> Result<(AccountId, Digest, Digest, Digest, Felt), AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE * WORD_SIZE {
        return Err(AccountError::StubDataIncorrectLength(
            elements.len(),
            ACCT_DATA_MEM_SIZE * WORD_SIZE,
        ));
    }

    let id = AccountId::try_from(elements[ACCT_ID_IDX])?;
    let nonce = elements[ACCT_NONCE_IDX];
    let vault_root = TryInto::<Word>::try_into(
        &elements[ACCT_VAULT_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_VAULT_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("word is correct size")
    .into();
    let storage_root = TryInto::<Word>::try_into(
        &elements[ACCT_STORAGE_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_STORAGE_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("ord is correct size")
    .into();
    let code_root = TryInto::<Word>::try_into(
        &elements[ACCT_CODE_ROOT_OFFSET as usize * WORD_SIZE
            ..(ACCT_CODE_ROOT_OFFSET as usize + 1) * WORD_SIZE],
    )
    .expect("word is correct size")
    .into();

    Ok((id, vault_root, storage_root, code_root, nonce))
}
