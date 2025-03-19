use crate::{
    Digest, EMPTY_WORD,
    block::{BlockHeader, BlockNumber},
    errors::AccountIdError,
};

// ACCOUNT ID ANCHOR
// ================================================================================================

/// The anchor of an [`AccountId`](crate::account::AccountId). See the type's documentation for
/// details on anchors.
///
/// This type is recommended to be created from a reference to a [`BlockHeader`] via the `TryFrom`
/// impl.
///
/// # Constraints
///
/// This type enforces the following constraints.
/// - The `anchor_block_number` % 2^[`BlockNumber::EPOCH_LENGTH_EXPONENT`] must be zero. In other
///   words, the block number must a multiple of 2^[`BlockNumber::EPOCH_LENGTH_EXPONENT`].
/// - The epoch derived from the `anchor_block_number` must be strictly less than [`u16::MAX`].
#[derive(Debug, Clone, Copy)]
pub struct AccountIdAnchor {
    epoch: u16,
    block_commitment: Digest,
}

impl AccountIdAnchor {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// A "pre-genesis" [`AccountIdAnchor`] which can be used to anchor accounts created in the
    /// genesis block.
    ///
    /// This anchor should only be used for accounts included in the genesis state, but should not
    /// be used as actual anchors in a running network. The reason is that this anchor has the same
    /// `epoch` as the genesis block will have (epoch `0`). However, the genesis block will have a
    /// different block_commitment than this anchor ([`EMPTY_WORD`]) and so any account ID that
    /// would use this anchor would be rejected as invalid by the transaction kernel.
    pub const PRE_GENESIS: Self = Self {
        epoch: 0,
        block_commitment: Digest::new(EMPTY_WORD),
    };

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`AccountIdAnchor`] from the provided `anchor_block_number` and
    /// `anchor_block_commitment`.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the anchor constraints are not met. See the [type
    /// documentation](AccountIdAnchor) for details.
    pub fn new(
        anchor_block_number: BlockNumber,
        anchor_block_commitment: Digest,
    ) -> Result<Self, AccountIdError> {
        if anchor_block_number.as_u32() & 0x0000_ffff != 0 {
            return Err(AccountIdError::AnchorBlockMustBeEpochBlock);
        }

        let anchor_epoch = anchor_block_number.block_epoch();

        if anchor_epoch == u16::MAX {
            return Err(AccountIdError::AnchorEpochMustNotBeU16Max);
        }

        Ok(Self {
            epoch: anchor_epoch,
            block_commitment: anchor_block_commitment,
        })
    }

    /// Creates a new [`AccountIdAnchor`] from the provided `anchor_epoch` and
    /// `anchor_block_commitment` without validation.
    ///
    /// # Warning
    ///
    /// The caller must ensure validity of the `anchor_epoch`, in particular the correctness of the
    /// relationship between the `anchor_epoch` and the provided `anchor_block_commitment`.
    ///
    /// # Panics
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if the
    /// `anchor_epoch` is [`u16::MAX`].
    pub fn new_unchecked(anchor_epoch: u16, anchor_block_commitment: Digest) -> Self {
        debug_assert_ne!(anchor_epoch, u16::MAX, "anchor epoch cannot be u16::MAX");

        Self {
            epoch: anchor_epoch,
            block_commitment: anchor_block_commitment,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the epoch of this anchor.
    pub fn epoch(self) -> u16 {
        self.epoch
    }

    /// Returns the block commitment of this anchor.
    pub fn block_commitment(self) -> Digest {
        self.block_commitment
    }
}

// CONVERSIONS TO ACCOUNT ID ANCHOR
// ================================================================================================

impl TryFrom<&BlockHeader> for AccountIdAnchor {
    type Error = AccountIdError;

    /// Extracts the [`BlockHeader::block_num`] and [`BlockHeader::commitment`] from the provided
    /// `block_header` and tries to convert it to an [`AccountIdAnchor`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the anchor constraints are not met. See the [type
    /// documentation](AccountIdAnchor) for details.
    fn try_from(block_header: &BlockHeader) -> Result<Self, Self::Error> {
        Self::new(block_header.block_num(), block_header.commitment())
    }
}
