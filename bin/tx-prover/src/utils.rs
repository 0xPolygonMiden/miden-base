use figment::{
    providers::{Format, Toml},
    Figment,
};

use crate::{CliConfig, PROVER_SERVICE_CONFIG_FILE_NAME};

/// Loads config file from current directory and default filename and returns it
///
/// This function will look for the configuration file at the provided path. If the path is
/// relative, searches in parent directories all the way to the root as well.
pub(crate) fn load_config_from_file() -> Result<CliConfig, String> {
    let mut current_dir = std::env::current_dir().map_err(|err| err.to_string())?;
    current_dir.push(PROVER_SERVICE_CONFIG_FILE_NAME);
    let config_path = current_dir.as_path();

    Figment::from(Toml::file(config_path))
        .extract()
        .map_err(|err| format!("Failed to load {} config file: {err}", config_path.display()))
}
