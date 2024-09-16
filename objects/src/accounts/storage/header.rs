use alloc::vec::Vec;

use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use super::{AccountStorage, StorageSlotType, Word};

// ACCOUNT STORAGE HEADER
// ================================================================================================

/// Account storage header is a lighter version of the [AccountStorage] storing
/// only the [StorageSlotType] and associated values of the [super::StorageSlot]s
/// contained in the storage.
///
/// The use of a header is useful in the situation where the storage is heavy (i.g. multiple Mb's),
/// and should be enough to execute most transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountStorageHeader {
    slots: Vec<(StorageSlotType, Word)>,
}

impl AccountStorageHeader {
    /// Returns a reference to the storage header slots.
    pub fn slots(&self) -> &Vec<(StorageSlotType, Word)> {
        &self.slots
    }
}

impl From<AccountStorage> for AccountStorageHeader {
    fn from(value: AccountStorage) -> Self {
        let slots = value
            .slots()
            .iter()
            .map(|storage_slot| (storage_slot.slot_type(), storage_slot.value()))
            .collect();

        AccountStorageHeader { slots }
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountStorageHeader {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        let len = self.slots.len() as u8;
        target.write_u8(len);
        target.write_many(self.slots())
    }
}

impl Deserializable for AccountStorageHeader {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let len = source.read_u8()?;
        let slots = source.read_many(len as usize)?;
        Ok(Self { slots })
    }
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use vm_core::{
        utils::{Deserializable, Serializable},
        Felt,
    };

    use super::AccountStorageHeader;
    use crate::accounts::{AccountStorage, StorageSlotType};

    #[test]
    fn test_from_account_storage() {
        // create new storage header from AccountStorage
        let slots = vec![
            (StorageSlotType::Value, [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
            (StorageSlotType::Value, [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)]),
            (
                StorageSlotType::Map,
                [
                    Felt::new(12405212884040084310),
                    Felt::new(17614307840949763446),
                    Felt::new(6101527485586301500),
                    Felt::new(14442045877206841081),
                ],
            ),
        ];

        let expected_header = AccountStorageHeader { slots };
        let account_storage = AccountStorage::mock();

        assert_eq!(expected_header, AccountStorageHeader::from(account_storage))
    }

    #[test]
    fn test_serde_account_storage_header() {
        // create new storage header
        let storage = AccountStorage::mock();
        let storage_header = AccountStorageHeader::from(storage);

        // serde storage header
        let bytes = storage_header.to_bytes();
        let deserialized = AccountStorageHeader::read_from_bytes(&bytes).unwrap();

        // assert deserialized == storage header
        assert_eq!(storage_header, deserialized);
    }
}
