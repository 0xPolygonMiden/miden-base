use vm_core::utils::{ByteReader, ByteWriter, Deserializable, Serializable};
use vm_processor::DeserializationError;

use crate::AccountError;

use super::{Digest, Felt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AccountProcedure {
    mast_root: Digest,
    storage_offset: u16,
}

impl AccountProcedure {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// The number of field elements needed to represent an [AccountProcedure]
    pub const NUM_ELEMENTS_PER_PROC: usize = 8;

    /// Returns a new instance of an [AccountProcedure]
    pub fn new(mast_root: Digest, storage_offset: u16) -> Self {
        Self { mast_root, storage_offset }
    }

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
        let storage_offset: u16 = value[4]
            .try_into()
            .map_err(|_| AccountError::AccountCodeProcedureInvalidStorageOffset)?;

        Ok(Self { mast_root, storage_offset })
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for AccountProcedure {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write(self.mast_root());
        target.write_u16(self.storage_offset());
    }
}

impl Deserializable for AccountProcedure {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let mast_root: Digest = source.read()?;
        let storage_offset = source.read_u16()?;

        Ok(Self::new(mast_root, storage_offset))
    }
}

#[cfg(test)]
mod tests {

    use assembly::{ast::ModuleAst, Assembler};
    use miden_crypto::utils::{Deserializable, Serializable};

    use crate::accounts::{AccountCode, AccountProcedure};

    const CODE: &str = "
        export.foo
            push.1 push.2 mul
        end

        export.bar
            push.1 push.2 add
        end
    ";

    fn make_account_code() -> AccountCode {
        let mut module = ModuleAst::parse(CODE).unwrap();
        // clears are needed since they're not serialized for account code
        module.clear_imports();
        module.clear_locations();
        AccountCode::new(module, &Assembler::default()).unwrap()
    }

    #[test]
    fn test_serde_account_procedure() {
        let account_code = make_account_code();

        let serialized = account_code.procedures()[0].to_bytes();
        let deserialized = AccountProcedure::read_from_bytes(&serialized).unwrap();

        assert_eq!(deserialized, account_code.procedures()[0]);
    }
}
