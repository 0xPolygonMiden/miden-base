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
/// The info included the MAST root of the procedure, the storage offset applied to all account
/// storage-related accesses made by this procedure and the storage size allowed to be accessed
/// by this procedure.
///
/// The offset is applied to any accesses made from within the procedure to the associated
/// account's storage. For example, if storage offset for a procedure is set ot 1, a call
/// to the account::get_item(storage_slot=4) made from this procedure would actually access
/// storage slot with index 5.
///
/// The size is used to limit how many storage slots a given procedure can access in the associated
/// account's storage. For example, if storage size for a procedure is set to 3, the procedure will
/// be bounded to access storage slots in the range [storage_offset, storage_offset + 3 - 1].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccountProcedureInfo {
    mast_root: Digest,
    storage_offset: u8,
    storage_size: u8,
}

impl AccountProcedureInfo {
    /// The number of field elements needed to represent an [AccountProcedureInfo] in kernel memory.
    pub const NUM_ELEMENTS_PER_PROC: usize = 8;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new instance of an [AccountProcedureInfo].
    ///
    /// # Panics
    /// Panics if `storage_size` is 0 and `storage_offset` is not 0.
    pub fn new(mast_root: Digest, storage_offset: u8, storage_size: u8) -> Self {
        if storage_size == 0 && storage_offset != 0 {
            panic!("storage_offset must be 0 when storage_size is 0");
        }

        Self { mast_root, storage_offset, storage_size }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a reference to the procedure's mast root.
    pub fn mast_root(&self) -> &Digest {
        &self.mast_root
    }

    /// Returns the procedure's storage offset.
    pub fn storage_offset(&self) -> u8 {
        self.storage_offset
    }

    /// Returns the procedure's storage size.
    pub fn storage_size(&self) -> u8 {
        self.storage_size
    }
}

impl From<AccountProcedureInfo> for [Felt; 8] {
    fn from(value: AccountProcedureInfo) -> Self {
        let mut result = [Felt::ZERO; 8];

        // copy mast_root into first 4 elements
        result[0..4].copy_from_slice(value.mast_root().as_elements());

        // copy the storage offset into value[4]
        result[4] = Felt::from(value.storage_offset);

        // copy the storage size into value[7]
        result[7] = Felt::from(value.storage_size);

        result
    }
}

impl TryFrom<[Felt; 8]> for AccountProcedureInfo {
    type Error = AccountError;

    fn try_from(value: [Felt; 8]) -> Result<Self, Self::Error> {
        // get mast_root from first 4 elements
        let mast_root = Digest::from(<[Felt; 4]>::try_from(&value[0..4]).unwrap());

        // get storage_offset form value[4]
        let storage_offset: u8 = value[4]
            .try_into()
            .map_err(|_| AccountError::AccountCodeProcedureInvalidStorageOffset)?;

        // Check if the next two elements are zero
        if value[5] != Felt::ZERO || value[6] != Felt::ZERO {
            return Err(AccountError::AccountCodeProcedureInvalidPadding);
        }

        // get storage_size form value[7]
        let storage_size: u8 = value[7]
            .try_into()
            .map_err(|_| AccountError::AccountCodeProcedureInvalidStorageSize)?;

        Ok(Self { mast_root, storage_offset, storage_size })
    }
}

impl Serializable for AccountProcedureInfo {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.mast_root);
        target.write_u8(self.storage_offset);
        target.write_u8(self.storage_size)
    }
}

impl Deserializable for AccountProcedureInfo {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let mast_root: Digest = source.read()?;
        let storage_offset = source.read_u8()?;
        let storage_size = source.read_u8()?;

        Ok(Self::new(mast_root, storage_offset, storage_size))
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {

    use miden_crypto::utils::{Deserializable, Serializable};
    use vm_core::Felt;

    use crate::accounts::{AccountCode, AccountProcedureInfo};

    #[test]
    fn test_from_to_account_procedure() {
        let account_code = AccountCode::mock();

        let procedure = account_code.procedures()[0].clone();

        // from procedure to [Felt; 8]
        let felts: [Felt; 8] = procedure.clone().into();

        // try_from [Felt; 8] to procedure
        let final_procedure: AccountProcedureInfo = felts.try_into().unwrap();

        assert_eq!(procedure, final_procedure);
    }

    #[test]
    fn test_serde_account_procedure() {
        let account_code = AccountCode::mock();

        let serialized = account_code.procedures()[0].to_bytes();
        let deserialized = AccountProcedureInfo::read_from_bytes(&serialized).unwrap();

        assert_eq!(account_code.procedures()[0], deserialized);
    }
}
