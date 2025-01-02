use std::{fs::File, io::Write};

use clap::Parser;

use crate::{commands::ProxyConfig, utils::PROVING_SERVICE_CONFIG_FILE_NAME};

/// Creates a config file for the proxy.
#[derive(Debug, Parser)]
pub struct Init;

impl Init {
    /// Creates a config file for the proxy.
    ///
    /// This method will create a new config file names
    /// [miden_tx_prover::PROVER_SERVICE_CONFIG_FILE_NAME] in the current working directory with
    /// default values.
    pub fn execute(&self) -> Result<(), String> {
        let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
        current_dir.push(PROVING_SERVICE_CONFIG_FILE_NAME);

        if current_dir.exists() {
            return Err(format!(
                "The file \"{}\" already exists in the working directory.",
                PROVING_SERVICE_CONFIG_FILE_NAME
            )
            .to_string());
        }

        let cli_config = ProxyConfig::default();

        let config_as_toml_string = toml::to_string_pretty(&cli_config)
            .map_err(|err| format!("Error formatting config: {err}"))?;

        let mut file_handle = File::options()
            .write(true)
            .create_new(true)
            .open(&current_dir)
            .map_err(|err| format!("Error opening the file: {err}"))?;

        file_handle
            .write(config_as_toml_string.as_bytes())
            .map_err(|err| format!("Error writing to file: {err}"))?;

        println!("Config file successfully created at: {:?}", current_dir);

        Ok(())
    }
}
