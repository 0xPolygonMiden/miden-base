use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::commands::ProxyConfig;

// ADD WORKERS
// ================================================================================================

/// Add workers to the proxy
#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct AddWorkers {
    workers: Vec<String>,
}

// REMOVE WORKERS
// ================================================================================================

/// Remove workers from the proxy
#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct RemoveWorkers {
    workers: Vec<String>,
}

// UPDATE WORKERS
// ================================================================================================

/// Action to perform on the workers
#[derive(clap::ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Add,
    Remove,
}

/// Update workers in the proxy performing the specified [Action]
#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct UpdateWorkers {
    pub action: Action,
    pub workers: Vec<String>,
}

impl UpdateWorkers {
    /// Makes a requests to the proxy to update the workers.
    ///
    /// It works by sending a GET request to the proxy with the query parameters. The query
    /// parameters are serialized from the struct fields.
    ///
    /// This method will work only if the proxy is running and the user is in the same computer as
    /// the proxy, since the proxy checks for the source IP address and checks that the sender is
    /// localhost.
    ///
    /// The request will return the new number of workers in the X-Worker-Count header.
    ///
    /// # Errors
    /// - If a tokio runtime cannot be created.
    /// - If the query parameters cannot be serialized.
    /// - If the request fails.
    /// - If the status code is not successful.
    /// - If the X-Worker-Count header is missing.
    pub async fn execute(&self) -> Result<(), String> {
        // Define a runtime

        let query_params = serde_qs::to_string(&self).map_err(|err| err.to_string())?;

        println!("Action: {:?}, with workers: {:?}", self.action, self.workers);

        // Get the proxy url from the configuration file.
        let proxy_config = ProxyConfig::load_config_from_file()?;

        // Create the full URL
        let url = format!(
            "http://{}:{}?{}",
            proxy_config.host, proxy_config.workers_update_port, query_params
        );

        // Create an HTTP/2 client
        let client = Client::builder().http1_only().build().map_err(|err| err.to_string())?;

        // Make the request
        let response = client.get(url).send().await.map_err(|err| err.to_string())?;

        // Check status code
        if !response.status().is_success() {
            return Err(format!("Request failed with status code: {}", response.status()));
        }

        // Read the X-Worker-Count header
        let workers_count = response
            .headers()
            .get("X-Worker-Count")
            .ok_or("Missing X-Worker-Count header")?
            .to_str()
            .map_err(|err| err.to_string())?;

        println!("New number of workers: {}", workers_count);

        Ok(())
    }
}

// CONVERSIONS
// ================================================================================================

impl From<RemoveWorkers> for UpdateWorkers {
    fn from(remove_workers: RemoveWorkers) -> Self {
        UpdateWorkers {
            action: Action::Remove,
            workers: remove_workers.workers,
        }
    }
}

impl From<AddWorkers> for UpdateWorkers {
    fn from(add_workers: AddWorkers) -> Self {
        UpdateWorkers {
            action: Action::Add,
            workers: add_workers.workers,
        }
    }
}
