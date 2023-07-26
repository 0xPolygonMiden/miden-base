use super::{
    Account, AccountDelta, AccountError, AccountId, AccountStorage, AccountStub, BTreeMap,
    ConsumedNotes, CreatedNotes, Digest, Felt, MerkleStore, Program, StackOutputs,
    TransactionResultError, TransactionWitness, TryFromVmResult, Vec, Word, WORD_SIZE,
};
use crate::accounts::AccountStorageDelta;
use crypto::merkle::{merkle_tree_delta, MerkleStoreDelta, MerkleTreeDelta, NodeIndex};
use miden_core::utils::group_slice_elements;
use miden_lib::memory::{
    ACCT_CODE_ROOT_OFFSET, ACCT_DATA_MEM_SIZE, ACCT_ID_AND_NONCE_OFFSET, ACCT_ID_IDX,
    ACCT_NONCE_IDX, ACCT_STORAGE_ROOT_OFFSET, ACCT_VAULT_ROOT_OFFSET,
};
use miden_processor::{AdviceInputs, RecAdviceProvider};

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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        initial_account: Account,
        consumed_notes: ConsumedNotes,
        block_hash: Digest,
        program: Program,
        tx_script_root: Option<Digest>,
        advice_provider: RecAdviceProvider,
        stack_outputs: StackOutputs,
    ) -> Result<Self, TransactionResultError> {
        // finalize the advice recorder
        let (witness, stack, map, store) = advice_provider.finalize();

        // parse transaction results
        let final_account_stub =
            FinalAccountStub::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;
        let created_notes = CreatedNotes::try_from_vm_result(&stack_outputs, &stack, &map, &store)?;

        // extract the account storage delta
        let storage_delta =
            extract_account_storage_delta(&store, &initial_account, &final_account_stub)?;

        // extract the nonce delta
        let nonce_delta = if initial_account.nonce() != final_account_stub.0.nonce() {
            Some(final_account_stub.0.nonce())
        } else {
            None
        };

        // TODO: implement vault delta extraction
        let vault_delta = MerkleTreeDelta::new(0);

        // construct the account delta
        let account_delta = AccountDelta {
            code: None,
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
            advice_witness: witness,
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

// FINAL ACCOUNT STUB
// ================================================================================================
/// [FinalAccountStub] represents a stub of an account after a transaction has been executed.
pub struct FinalAccountStub(pub AccountStub);

impl TryFromVmResult for FinalAccountStub {
    type Error = TransactionResultError;

    fn try_from_vm_result(
        stack_outputs: &StackOutputs,
        _advice_stack: &[Felt],
        advice_map: &BTreeMap<[u8; 32], Vec<Felt>>,
        _merkle_store: &MerkleStore,
    ) -> Result<Self, Self::Error> {
        const FINAL_ACCOUNT_HASH_WORD_IDX: usize = 1;

        let final_account_hash: Word =
            stack_outputs.stack()[FINAL_ACCOUNT_HASH_WORD_IDX * WORD_SIZE
                ..(FINAL_ACCOUNT_HASH_WORD_IDX + 1) * WORD_SIZE]
                .iter()
                .rev()
                .map(|x| Felt::from(*x))
                .collect::<Vec<_>>()
                .try_into()
                .expect("word size is correct");
        let final_account_hash: Digest = final_account_hash.into();

        // extract final account data from the advice map
        let final_account_data = group_slice_elements::<Felt, WORD_SIZE>(
            advice_map
                .get(&final_account_hash.as_bytes())
                .ok_or(TransactionResultError::FinalAccountDataNotFound)?,
        );
        let stub = parse_stub(final_account_data)
            .map_err(TransactionResultError::FinalAccountStubDataInvalid)?;

        Ok(Self(stub))
    }
}

/// Parses the stub account data returned by the VM into individual account component commitments.
/// Returns a tuple of account ID, vault root, storage root, code root, and nonce.
fn parse_stub(elements: &[Word]) -> Result<AccountStub, AccountError> {
    if elements.len() != ACCT_DATA_MEM_SIZE {
        return Err(AccountError::StubDataIncorrectLength(elements.len(), ACCT_DATA_MEM_SIZE));
    }

    let id = AccountId::try_from(elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_ID_IDX])?;
    let nonce = elements[ACCT_ID_AND_NONCE_OFFSET as usize][ACCT_NONCE_IDX];
    let vault_root = elements[ACCT_VAULT_ROOT_OFFSET as usize].into();
    let storage_root = elements[ACCT_STORAGE_ROOT_OFFSET as usize].into();
    let code_root = elements[ACCT_CODE_ROOT_OFFSET as usize].into();

    Ok(AccountStub::new(id, nonce, vault_root, storage_root, code_root))
}

// ACCOUNT STORAGE DELTA
// ================================================================================================
/// Extracts account storage delta between the `initial_account` and `final_account_stub` from the
/// provided `MerkleStore`
fn extract_account_storage_delta(
    store: &MerkleStore,
    initial_account: &Account,
    final_account_stub: &FinalAccountStub,
) -> Result<AccountStorageDelta, TransactionResultError> {
    // extract storage slots delta
    let slots_delta = merkle_tree_delta(
        initial_account.storage().root(),
        final_account_stub.0.storage_root(),
        AccountStorage::STORAGE_TREE_DEPTH,
        store,
    )
    .map_err(TransactionResultError::ExtractAccountStorageSlotsDeltaFailed)?;

    // extract child deltas
    let mut store_delta = vec![];
    for (slot, new_value) in slots_delta.updated_slots() {
        // if a slot was updated, check if it was originally a Merkle root of a Merkle tree
        let leaf = store
            .get_node(
                initial_account.storage().root(),
                NodeIndex::new_unchecked(AccountStorage::STORAGE_TREE_DEPTH, *slot),
            )
            .expect("storage slut must exist");
        // if a slot was a Merkle root then extract the delta.  We assume the tree is a SMT of depth 64.
        if store.get_node(leaf, NodeIndex::new_unchecked(0, 0)).is_ok() {
            let child_delta = merkle_tree_delta(leaf, (*new_value).into(), 64, store)
                .map_err(TransactionResultError::ExtractAccountStorageStoreDeltaFailed)?;
            store_delta.push((leaf, child_delta));
        }
    }

    // construct storage delta
    let storage_delta = AccountStorageDelta {
        slots_delta,
        store_delta: MerkleStoreDelta(store_delta),
    };

    Ok(storage_delta)
}
