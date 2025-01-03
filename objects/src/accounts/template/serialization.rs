use vm_core::utils::Serializable;

use super::AccountComponentTemplate;


#[cfg(feature = "std")]
impl Serializable for AccountComponentTemplate {
    fn write_into<W: vm_core::utils::ByteWriter>(&self, target: &mut W) {
        // Since `Self::new` ensures valid TOML, unwrap is safe here.
        let config_toml = toml::to_string(&self.metadata)
            .expect("Failed to serialize AccountComponentTemplate to TOML");
        target.write(config_toml);
        target.write(&self.library);
    }
}

#[cfg(feature = "std")]
impl Deserializable for AccountComponentTemplate {
    fn read_from<R: vm_core::utils::ByteReader>(
        source: &mut R,
    ) -> Result<Self, vm_processor::DeserializationError> {
        // Read and deserialize the configuration from a TOML string.
        let config_str = String::read_from(source)?;
        let config: ComponentMetadata = toml::from_str(&config_str)
            .map_err(|e| vm_processor::DeserializationError::InvalidValue(e.to_string()))?;
        let library = Library::read_from(source)?;

        let package = AccountComponentTemplate::new(config, library);
        Ok(package)
    }
}