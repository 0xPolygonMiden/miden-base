use alloc::{string::ToString, vec::Vec};

use miden_verifier::ExecutionProof;

use super::{InputNote, ToInputNoteCommitments};
use crate::{
    account::delta::AccountUpdateDetails,
    block::BlockNumber,
    note::NoteHeader,
    transaction::{
        AccountId, Digest, InputNotes, Nullifier, OutputNote, OutputNotes, TransactionId,
    },
    utils::serde::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    ProvenTransactionError, ACCOUNT_UPDATE_MAX_SIZE,
};

// PROVEN TRANSACTION
// ================================================================================================

/// Result of executing and proving a transaction. Contains all the data required to verify that a
/// transaction was executed correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenTransaction {
    /// A unique identifier for the transaction, see [TransactionId] for additional details.
    id: TransactionId,

    /// Account update data.
    account_update: TxAccountUpdate,

    /// Committed details of all notes consumed by the transaction.
    input_notes: InputNotes<InputNoteCommitment>,

    /// Notes created by the transaction. For private notes, this will contain only note headers,
    /// while for public notes this will also contain full note details.
    output_notes: OutputNotes,

    /// [`BlockNumber`] of the transaction's reference block.
    ///
    /// This is not needed for proving the transaction, but it is useful for the node to lookup the
    /// block.
    block_num: BlockNumber,

    /// The block hash of the last known block at the time the transaction was executed.
    block_ref: Digest,

    /// The block number by which the transaction will expire, as defined by the executed scripts.
    expiration_block_num: BlockNumber,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: ExecutionProof,
}

impl ProvenTransaction {
    /// Returns unique identifier of this transaction.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Returns ID of the account against which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.account_update.account_id()
    }

    /// Returns the account update details.
    pub fn account_update(&self) -> &TxAccountUpdate {
        &self.account_update
    }

    /// Returns a reference to the notes consumed by the transaction.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
        &self.input_notes
    }

    /// Returns a reference to the notes produced by the transaction.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.output_notes
    }

    /// Returns the proof of the transaction.
    pub fn proof(&self) -> &ExecutionProof {
        &self.proof
    }

    /// Returns the number of the reference block the transaction was executed against.
    pub fn block_num(&self) -> BlockNumber {
        self.block_num
    }

    /// Returns the block reference the transaction was executed against.
    pub fn block_ref(&self) -> Digest {
        self.block_ref
    }

    /// Returns an iterator of the headers of unauthenticated input notes in this transaction.
    pub fn get_unauthenticated_notes(&self) -> impl Iterator<Item = &NoteHeader> {
        self.input_notes.iter().filter_map(|note| note.header())
    }

    /// Returns the block number at which the transaction will expire.
    pub fn expiration_block_num(&self) -> BlockNumber {
        self.expiration_block_num
    }

    /// Returns an iterator over the nullifiers of all input notes in this transaction.
    ///
    /// This includes both authenticated and unauthenticated notes.
    pub fn get_nullifiers(&self) -> impl Iterator<Item = Nullifier> + '_ {
        self.input_notes.iter().map(InputNoteCommitment::nullifier)
    }

    // HELPER METHODS
    // --------------------------------------------------------------------------------------------

    fn validate(self) -> Result<Self, ProvenTransactionError> {
        if self.account_id().is_public() {
            self.account_update.validate()?;

            let is_new_account = self.account_update.init_state_hash() == Digest::default();
            match self.account_update.details() {
                AccountUpdateDetails::Private => {
                    return Err(ProvenTransactionError::OnChainAccountMissingDetails(
                        self.account_id(),
                    ))
                },
                AccountUpdateDetails::New(ref account) => {
                    if !is_new_account {
                        return Err(
                            ProvenTransactionError::ExistingOnChainAccountRequiresDeltaDetails(
                                self.account_id(),
                            ),
                        );
                    }
                    if account.id() != self.account_id() {
                        return Err(ProvenTransactionError::AccountIdMismatch {
                            tx_account_id: self.account_id(),
                            details_account_id: account.id(),
                        });
                    }
                    if account.hash() != self.account_update.final_state_hash() {
                        return Err(ProvenTransactionError::AccountFinalHashMismatch {
                            tx_final_hash: self.account_update.final_state_hash(),
                            details_hash: account.hash(),
                        });
                    }
                },
                AccountUpdateDetails::Delta(_) => {
                    if is_new_account {
                        return Err(ProvenTransactionError::NewOnChainAccountRequiresFullDetails(
                            self.account_id(),
                        ));
                    }
                },
            }
        } else if !self.account_update.is_private() {
            return Err(ProvenTransactionError::OffChainAccountWithDetails(self.account_id()));
        }

        Ok(self)
    }
}

impl Serializable for ProvenTransaction {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_update.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.block_num.write_into(target);
        self.block_ref.write_into(target);
        self.expiration_block_num.write_into(target);
        self.proof.write_into(target);
    }
}

impl Deserializable for ProvenTransaction {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let account_update = TxAccountUpdate::read_from(source)?;

        let input_notes = <InputNotes<InputNoteCommitment>>::read_from(source)?;
        let output_notes = OutputNotes::read_from(source)?;

        let block_num = BlockNumber::read_from(source)?;
        let block_ref = Digest::read_from(source)?;
        let expiration_block_num = BlockNumber::read_from(source)?;
        let proof = ExecutionProof::read_from(source)?;

        let id = TransactionId::new(
            account_update.init_state_hash(),
            account_update.final_state_hash(),
            input_notes.commitment(),
            output_notes.commitment(),
        );

        let proven_transaction = Self {
            id,
            account_update,
            input_notes,
            output_notes,
            block_num,
            block_ref,
            expiration_block_num,
            proof,
        };

        proven_transaction
            .validate()
            .map_err(|err| DeserializationError::InvalidValue(err.to_string()))
    }
}

// PROVEN TRANSACTION BUILDER
// ================================================================================================

/// Builder for a proven transaction.
#[derive(Clone, Debug)]
pub struct ProvenTransactionBuilder {
    /// ID of the account that the transaction was executed against.
    account_id: AccountId,

    /// The hash of the account before the transaction was executed.
    initial_account_hash: Digest,

    /// The hash of the account after the transaction was executed.
    final_account_hash: Digest,

    /// State changes to the account due to the transaction.
    account_update_details: AccountUpdateDetails,

    /// List of [InputNoteCommitment]s of all consumed notes by the transaction.
    input_notes: Vec<InputNoteCommitment>,

    /// List of [OutputNote]s of all notes created by the transaction.
    output_notes: Vec<OutputNote>,

    /// [`BlockNumber`] of the transaction's reference block.
    block_num: BlockNumber,

    /// Block [Digest] of the transaction's reference block.
    block_ref: Digest,

    /// The block number by which the transaction will expire, as defined by the executed scripts.
    expiration_block_num: BlockNumber,

    /// A STARK proof that attests to the correct execution of the transaction.
    proof: ExecutionProof,
}

impl ProvenTransactionBuilder {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a [ProvenTransactionBuilder] used to build a [ProvenTransaction].
    pub fn new(
        account_id: AccountId,
        initial_account_hash: Digest,
        final_account_hash: Digest,
        block_num: BlockNumber,
        block_ref: Digest,
        expiration_block_num: BlockNumber,
        proof: ExecutionProof,
    ) -> Self {
        Self {
            account_id,
            initial_account_hash,
            final_account_hash,
            account_update_details: AccountUpdateDetails::Private,
            input_notes: Vec::new(),
            output_notes: Vec::new(),
            block_num,
            block_ref,
            expiration_block_num,
            proof,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Sets the account's update details.
    pub fn account_update_details(mut self, details: AccountUpdateDetails) -> Self {
        self.account_update_details = details;
        self
    }

    /// Add notes consumed by the transaction.
    pub fn add_input_notes<I, T>(mut self, notes: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<InputNoteCommitment>,
    {
        self.input_notes.extend(notes.into_iter().map(|note| note.into()));
        self
    }

    /// Add notes produced by the transaction.
    pub fn add_output_notes<T>(mut self, notes: T) -> Self
    where
        T: IntoIterator<Item = OutputNote>,
    {
        self.output_notes.extend(notes);
        self
    }

    /// Builds the [ProvenTransaction].
    ///
    /// # Errors
    ///
    /// An error will be returned if an on-chain account is used without provided on-chain detail.
    /// Or if the account details, i.e. account ID and final hash, don't match the transaction.
    pub fn build(self) -> Result<ProvenTransaction, ProvenTransactionError> {
        let input_notes =
            InputNotes::new(self.input_notes).map_err(ProvenTransactionError::InputNotesError)?;
        let output_notes = OutputNotes::new(self.output_notes)
            .map_err(ProvenTransactionError::OutputNotesError)?;
        let id = TransactionId::new(
            self.initial_account_hash,
            self.final_account_hash,
            input_notes.commitment(),
            output_notes.commitment(),
        );
        let account_update = TxAccountUpdate::new(
            self.account_id,
            self.initial_account_hash,
            self.final_account_hash,
            self.account_update_details,
        );

        let proven_transaction = ProvenTransaction {
            id,
            account_update,
            input_notes,
            output_notes,
            block_num: self.block_num,
            block_ref: self.block_ref,
            expiration_block_num: self.expiration_block_num,
            proof: self.proof,
        };

        proven_transaction.validate()
    }
}

// TRANSACTION ACCOUNT UPDATE
// ================================================================================================

/// Describes the changes made to the account state resulting from a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxAccountUpdate {
    /// ID of the account updated by a transaction.
    account_id: AccountId,

    /// The hash of the account before a transaction was executed.
    ///
    /// Set to `Digest::default()` for new accounts.
    init_state_hash: Digest,

    /// The hash of the account state after a transaction was executed.
    final_state_hash: Digest,

    /// A set of changes which can be applied the account's state prior to the transaction to
    /// get the account state after the transaction. For private accounts this is set to
    /// [AccountUpdateDetails::Private].
    details: AccountUpdateDetails,
}

impl TxAccountUpdate {
    /// Returns a new [TxAccountUpdate] instantiated from the specified components.
    pub const fn new(
        account_id: AccountId,
        init_state_hash: Digest,
        final_state_hash: Digest,
        details: AccountUpdateDetails,
    ) -> Self {
        Self {
            account_id,
            init_state_hash,
            final_state_hash,
            details,
        }
    }

    /// Returns the ID of the updated account.
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the hash of the account's initial state.
    pub fn init_state_hash(&self) -> Digest {
        self.init_state_hash
    }

    /// Returns the hash of the account's after a transaction was executed.
    pub fn final_state_hash(&self) -> Digest {
        self.final_state_hash
    }

    /// Returns the description of the updates for public accounts.
    ///
    /// These descriptions can be used to build the new account state from the previous account
    /// state.
    pub fn details(&self) -> &AccountUpdateDetails {
        &self.details
    }

    /// Returns `true` if the account update details are for a private account.
    pub fn is_private(&self) -> bool {
        self.details.is_private()
    }

    /// Validates the following properties of the account update:
    ///
    /// - The size of the serialized account update does not exceed [`ACCOUNT_UPDATE_MAX_SIZE`].
    pub fn validate(&self) -> Result<(), ProvenTransactionError> {
        let account_update_size = self.details().get_size_hint();
        if account_update_size > ACCOUNT_UPDATE_MAX_SIZE as usize {
            Err(ProvenTransactionError::AccountUpdateSizeLimitExceeded {
                account_id: self.account_id(),
                update_size: account_update_size,
            })
        } else {
            Ok(())
        }
    }
}

impl Serializable for TxAccountUpdate {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.account_id.write_into(target);
        self.init_state_hash.write_into(target);
        self.final_state_hash.write_into(target);
        self.details.write_into(target);
    }
}

impl Deserializable for TxAccountUpdate {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        Ok(Self {
            account_id: AccountId::read_from(source)?,
            init_state_hash: Digest::read_from(source)?,
            final_state_hash: Digest::read_from(source)?,
            details: AccountUpdateDetails::read_from(source)?,
        })
    }
}

// INPUT NOTE COMMITMENT
// ================================================================================================

/// The commitment to an input note.
///
/// For notes authenticated by the transaction kernel, the commitment consists only of the note's
/// nullifier. For notes whose authentication is delayed to batch/block kernels, the commitment
/// also includes full note header (i.e., note ID and metadata).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputNoteCommitment {
    nullifier: Nullifier,
    header: Option<NoteHeader>,
}

impl InputNoteCommitment {
    /// Returns the nullifier of the input note committed to by this commitment.
    pub fn nullifier(&self) -> Nullifier {
        self.nullifier
    }

    /// Returns the header of the input committed to by this commitment.
    ///
    /// Note headers are present only for notes whose presence in the change has not yet been
    /// authenticated.
    pub fn header(&self) -> Option<&NoteHeader> {
        self.header.as_ref()
    }

    /// Returns true if this commitment is for a note whose presence in the chain has been
    /// authenticated.
    ///
    /// Authenticated notes are represented solely by their nullifiers and are missing the note
    /// header.
    pub fn is_authenticated(&self) -> bool {
        self.header.is_none()
    }
}

impl From<InputNote> for InputNoteCommitment {
    fn from(note: InputNote) -> Self {
        Self::from(&note)
    }
}

impl From<&InputNote> for InputNoteCommitment {
    fn from(note: &InputNote) -> Self {
        match note {
            InputNote::Authenticated { note, .. } => Self {
                nullifier: note.nullifier(),
                header: None,
            },
            InputNote::Unauthenticated { note } => Self {
                nullifier: note.nullifier(),
                header: Some(*note.header()),
            },
        }
    }
}

impl From<Nullifier> for InputNoteCommitment {
    fn from(nullifier: Nullifier) -> Self {
        Self { nullifier, header: None }
    }
}

impl ToInputNoteCommitments for InputNoteCommitment {
    fn nullifier(&self) -> Nullifier {
        self.nullifier
    }

    fn note_hash(&self) -> Option<Digest> {
        self.header.map(|header| header.hash())
    }
}

// SERIALIZATION
// ------------------------------------------------------------------------------------------------

impl Serializable for InputNoteCommitment {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.nullifier.write_into(target);
        self.header.write_into(target);
    }
}

impl Deserializable for InputNoteCommitment {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nullifier = Nullifier::read_from(source)?;
        let header = <Option<NoteHeader>>::read_from(source)?;

        Ok(Self { nullifier, header })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use miden_verifier::ExecutionProof;
    use vm_core::utils::Deserializable;
    use winter_air::proof::Proof;
    use winter_rand_utils::rand_array;

    use super::ProvenTransaction;
    use crate::{
        account::{
            delta::AccountUpdateDetails, AccountDelta, AccountId, AccountIdVersion,
            AccountStorageDelta, AccountStorageMode, AccountType, AccountVaultDelta,
            StorageMapDelta,
        },
        block::BlockNumber,
        testing::account_id::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        transaction::{ProvenTransactionBuilder, TxAccountUpdate},
        utils::Serializable,
        Digest, ProvenTransactionError, ACCOUNT_UPDATE_MAX_SIZE, EMPTY_WORD, ONE, ZERO,
    };

    fn check_if_sync<T: Sync>() {}
    fn check_if_send<T: Send>() {}

    /// [ProvenTransaction] being Sync is part of its public API and changing it is backwards
    /// incompatible.
    #[test]
    fn test_proven_transaction_is_sync() {
        check_if_sync::<ProvenTransaction>();
    }

    /// [ProvenTransaction] being Send is part of its public API and changing it is backwards
    /// incompatible.
    #[test]
    fn test_proven_transaction_is_send() {
        check_if_send::<ProvenTransaction>();
    }

    #[test]
    fn account_update_size_limit_not_exceeded() {
        // A small delta does not exceed the limit.
        let storage_delta = AccountStorageDelta::from_iters(
            [1, 2, 3, 4],
            [(2, [ONE, ONE, ONE, ONE]), (3, [ONE, ONE, ZERO, ONE])],
            [],
        );
        let delta =
            AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap();
        let details = AccountUpdateDetails::Delta(delta);
        TxAccountUpdate::new(
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            Digest::new(EMPTY_WORD),
            Digest::new(EMPTY_WORD),
            details,
        )
        .validate()
        .unwrap();
    }

    #[test]
    fn account_update_size_limit_exceeded() {
        let mut map = BTreeMap::new();
        // The number of entries in the map required to exceed the limit.
        // We divide by each entry's size which consists of a key (digest) and a value (word), both
        // 32 bytes in size.
        let required_entries = ACCOUNT_UPDATE_MAX_SIZE / (2 * 32);
        for _ in 0..required_entries {
            map.insert(Digest::new(rand_array()), rand_array());
        }
        let storage_delta = StorageMapDelta::new(map);

        // A delta that exceeds the limit returns an error.
        let storage_delta = AccountStorageDelta::from_iters([], [], [(4, storage_delta)]);
        let delta =
            AccountDelta::new(storage_delta, AccountVaultDelta::default(), Some(ONE)).unwrap();
        let details = AccountUpdateDetails::Delta(delta);
        let details_size = details.get_size_hint();

        let err = TxAccountUpdate::new(
            AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap(),
            Digest::new(EMPTY_WORD),
            Digest::new(EMPTY_WORD),
            details,
        )
        .validate()
        .unwrap_err();

        assert!(
            matches!(err, ProvenTransactionError::AccountUpdateSizeLimitExceeded { update_size, .. } if update_size == details_size)
        );
    }

    #[test]
    fn test_proven_tx_serde_roundtrip() {
        let account_id = AccountId::dummy(
            [1; 15],
            AccountIdVersion::Version0,
            AccountType::FungibleFaucet,
            AccountStorageMode::Private,
        );
        let initial_account_hash =
            [2; 32].try_into().expect("failed to create initial account hash");
        let final_account_hash = [3; 32].try_into().expect("failed to create final account hash");
        let block_num = BlockNumber::from(1);
        let block_ref = Digest::default();
        let expiration_block_num = BlockNumber::from(2);
        let proof = ExecutionProof::new(Proof::new_dummy(), Default::default());

        let tx = ProvenTransactionBuilder::new(
            account_id,
            initial_account_hash,
            final_account_hash,
            block_num,
            block_ref,
            expiration_block_num,
            proof,
        )
        .build()
        .expect("failed to build proven transaction");

        let deserialized = ProvenTransaction::read_from_bytes(&tx.to_bytes()).unwrap();

        assert_eq!(tx, deserialized);
    }
}
