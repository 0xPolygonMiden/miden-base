use crate::AccountError;

use super::{Digest, Felt};

pub struct AccountProcedure {
    mast_root: Digest,
    storage_offset: u16,
}

impl AccountProcedure {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The number of elements needed to represent an [AccountProcedure]
    pub const NUM_ELEMENTS_PER_PROC: usize = 8;

    /// Returns a reference to the procedure's mast_root
    pub fn mast_root(&self) -> &Digest {
        &self.mast_root
    }

    /// Returns a reference to the procedure's storage_offset
    pub fn storage_offset(&self) -> u16 {
        self.storage_offset
    }
}

impl TryFrom<[Felt; 8]> for AccountProcedure {
    type Error = AccountError;

    fn try_from(value: [Felt; 8]) -> Result<Self, Self::Error> {
        let mast_root = Digest::from(<[Felt; 4]>::try_from(&value[0..4]).unwrap());
        let storage_offset = u16::try_from(value[4].inner())
            .map_err(|_| AccountError::AccountCodeProcedureInvalidStorageOffset)?;

        Ok(Self { mast_root, storage_offset })
    }
}
