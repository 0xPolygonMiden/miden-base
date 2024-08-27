use vm_core::{
    utils::{ByteReader, ByteWriter, Deserializable, Serializable},
    FieldElement,
};
use vm_processor::DeserializationError;

use super::{Digest, Felt};
use crate::AccountError;

// ACCOUNT PROCEDURE INFO
// ================================================================================================

/// Information about a procedure exposed in a public account interface.
///
/// The info included the MAST root of the procedure and the storage offset applied to all account
/// storage-related accesses made by this procedure. For example, if storage offset is set ot 1, a
/// call to the account::get_item(storage_slot=4) made from this procedure would actually access
/// storage slot with index 5.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccountProcedureInfo {
    mast_root: Digest,
    storage_offset: u16,
}

impl AccountProcedureInfo {
    /// The number of field elements needed to represent an [AccountProcedureInfo] in kernel memory.
    pub const NUM_ELEMENTS_PER_PROC: usize = 8;

    /// Returns a new instance of an [AccountProcedureInfo].
    pub fn new(mast_root: Digest, storage_offset: u16) -> Self {
        Self { mast_root, storage_offset }
    }

    /// Returns a reference to the procedure's mast_root.
    pub fn mast_root(&self) -> &Digest {
        &self.mast_root
    }

    /// Returns a reference to the procedure's storage_offset.
    pub fn storage_offset(&self) -> u16 {
        self.storage_offset
    }
}

impl From<AccountProcedureInfo> for [Felt; 8] {
    fn from(value: AccountProcedureInfo) -> Self {
        let mut result = [Felt::ZERO; 8];

        // copy mast_root into first 4 elements
        result[0..4].copy_from_slice(value.mast_root().as_elements());

        // copy the storage offset into value[4]
        result[4] = Felt::from(value.storage_offset());

        result
    }
}

impl TryFrom<[Felt; 8]> for AccountProcedureInfo {
    type Error = AccountError;

    fn try_from(value: [Felt; 8]) -> Result<Self, Self::Error> {
        // get mast_root from first 4 elements
        let mast_root = Digest::from(<[Felt; 4]>::try_from(&value[0..4]).unwrap());

        // get storage_offset form value[4]
        let storage_offset: u16 = value[4]
            .try_into()
            .map_err(|_| AccountError::AccountCodeProcedureInvalidStorageOffset)?;

        // Check if the last three elements are zero
        if value[5..].iter().any(|&x| x != Felt::ZERO) {
            return Err(AccountError::AccountCodeProcedureInvalidPadding);
        }

        Ok(Self { mast_root, storage_offset })
    }
}

impl Serializable for AccountProcedureInfo {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.mast_root());
        target.write_u16(self.storage_offset());
    }
}

impl Deserializable for AccountProcedureInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let mast_root: Digest = source.read()?;
        let storage_offset = source.read_u16()?;

        Ok(Self::new(mast_root, storage_offset))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use miden_crypto::utils::{Deserializable, Serializable};

    use crate::accounts::{AccountCode, AccountProcedureInfo};

    #[test]
    fn test_serde_account_procedure() {
        let account_code = AccountCode::mock();

        let serialized = account_code.procedures()[0].to_bytes();
        let deserialized = AccountProcedureInfo::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized, account_code.procedures()[0]);
    }
}
