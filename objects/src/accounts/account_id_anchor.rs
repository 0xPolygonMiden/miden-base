use crate::{block::block_epoch_from_number, AccountError, BlockHeader, Digest, EMPTY_WORD};

// ACCOUNT ID ANCHOR
// ================================================================================================

/// The anchor of an [`AccountId`](crate::accounts::AccountId). See the type's documentation for
/// details on anchors.
///
/// This type is recommended to be created from a reference to a [`BlockHeader`] via the `TryFrom`
/// impl.
///
/// # Constraints
///
/// This type enforces the following constraints.
/// - The `anchor_block_number` % 2^[`BlockHeader::EPOCH_LENGTH_EXPONENT`] must be zero. In other
///   words, the block number must a multiple of 2^[`BlockHeader::EPOCH_LENGTH_EXPONENT`].
/// - The epoch derived from the `anchor_block_number` must be strictly less than [`u16::MAX`].
#[derive(Debug, Clone, Copy)]
pub struct AccountIdAnchor {
    epoch: u16,
    block_hash: Digest,
}

impl AccountIdAnchor {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// A "pre-genesis" [`AccountIdAnchor`] which can be used to anchor accounts created in the
    /// genesis block.
    pub const PRE_GENESIS: Self = Self {
        epoch: 0,
        block_hash: Digest::new(EMPTY_WORD),
    };

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`AccountIdAnchor`] from the provided `anchor_block_number` and
    /// `anchor_block_hash`.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the anchor constraints are not met. See the [type
    /// documentation](AccountIdAnchor) for details.
    pub fn new(anchor_block_number: u32, anchor_block_hash: Digest) -> Result<Self, AccountError> {
        if anchor_block_number & 0x0000_ffff != 0 {
            return Err(AccountError::AssumptionViolated(format!(
          "TODO: Make proper error: anchor block must be an epoch block, i.e. its block number must be a multiple of 2^{}",
          BlockHeader::EPOCH_LENGTH_EXPONENT)));
        }

        let anchor_epoch = block_epoch_from_number(anchor_block_number);

        if anchor_epoch == u16::MAX {
            return Err(AccountError::AssumptionViolated(format!(
                "TODO: Make proper error: anchor epoch cannot be {}",
                u16::MAX
            )));
        }

        Ok(Self {
            epoch: anchor_epoch,
            block_hash: anchor_block_hash,
        })
    }

    /// Creates a new [`AccountIdAnchor`] from the provided `anchor_epoch` and `anchor_block_hash`
    /// without validation.
    ///
    /// # Warning
    ///
    /// The caller must ensure validity of the `anchor_epoch`, in particular the correctness of the
    /// relationship between the `anchor_epoch` and the provided `anchor_block_hash`.
    ///
    /// # Panics
    ///
    /// If debug_assertions are enabled (e.g. in debug mode), this function panics if the
    /// `anchor_epoch` is [`u16::MAX`].
    pub fn new_unchecked(anchor_epoch: u16, anchor_block_hash: Digest) -> Self {
        debug_assert_ne!(anchor_epoch, u16::MAX, "anchor epoch cannot be u16::MAX");

        Self {
            epoch: anchor_epoch,
            block_hash: anchor_block_hash,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns the epoch of this anchor.
    pub fn epoch(self) -> u16 {
        self.epoch
    }

    /// Returns the block hash of this anchor.
    pub fn block_hash(self) -> Digest {
        self.block_hash
    }
}

// CONVERSIONS TO ACCOUNT ID ANCHOR
// ================================================================================================

impl TryFrom<&BlockHeader> for AccountIdAnchor {
    type Error = AccountError;

    /// Extracts the [`BlockHeader::block_num`] and [`BlockHeader::hash`] from the provided
    /// `block_header` and tries to convert it to an [`AccountIdAnchor`].
    ///
    /// # Errors
    ///
    /// Returns an error if any of the anchor constraints are not met. See the [type
    /// documentation](AccountIdAnchor) for details.
    fn try_from(block_header: &BlockHeader) -> Result<Self, Self::Error> {
        Self::new(block_header.block_num(), block_header.hash())
    }
}
