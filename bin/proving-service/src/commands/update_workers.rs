use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ADD WORKERS
// ================================================================================================

/// Add workers to the proxy
#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct AddWorkers {
    /// Workers to be added to the proxy.
    ///
    /// The workers are passed as host:port strings.
    #[clap(value_name = "WORKERS")]
    workers: Vec<String>,
    /// Host of the proxy.
    #[clap(long, default_value = "0.0.0.0", env = "MPS_HOST")]
    proxy_host: String,
    /// Port of the proxy endpoint to update workers.
    #[clap(long, default_value = "8083", env = "MPS_WORKERS_UPDATE_PORT")]
    proxy_update_workers_port: u64,
}

// REMOVE WORKERS
// ================================================================================================

/// Remove workers from the proxy
#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct RemoveWorkers {
    /// Workers to be removed from the proxy.
    ///
    /// The workers are passed as host:port strings.
    workers: Vec<String>,
    /// Host of the proxy.
    #[clap(long, default_value = "0.0.0.0", env = "MPS_HOST")]
    proxy_host: String,
    /// Port of the proxy endpoint to update workers.
    #[clap(long, default_value = "8083", env = "MPS_WORKERS_UPDATE_PORT")]
    proxy_update_workers_port: u64,
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
    pub proxy_host: String,
    pub proxy_update_workers_port: u64,
}

impl UpdateWorkers {
    /// Makes a requests to the update workers endpoint to update the workers.
    ///
    /// It works by sending a GET request to the proxy with the query parameters. The query
    /// parameters are serialized from the struct fields.
    ///
    /// It uses the host and port defined in the env vars or passed as parameter for the proxy.
    ///
    /// The request will return the new number of workers in the X-Worker-Count header.
    ///
    /// # Errors
    /// - If the query parameters cannot be serialized.
    /// - If the request fails.
    /// - If the status code is not successful.
    /// - If the X-Worker-Count header is missing.
    pub async fn execute(&self) -> Result<(), String> {
        // Define a runtime

        let query_params = serde_qs::to_string(&self).map_err(|err| err.to_string())?;

        println!("Action: {:?}, with workers: {:?}", self.action, self.workers);

        // Create the full URL
        let url = format!(
            "http://{}:{}?{}",
            self.proxy_host, self.proxy_update_workers_port, query_params
        );

        // Create an HTTP/2 client
        let client = Client::builder()
            .http2_prior_knowledge()
            .build()
            .map_err(|err| err.to_string())?;

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
            proxy_host: remove_workers.proxy_host,
            proxy_update_workers_port: remove_workers.proxy_update_workers_port,
        }
    }
}

impl From<AddWorkers> for UpdateWorkers {
    fn from(add_workers: AddWorkers) -> Self {
        UpdateWorkers {
            action: Action::Add,
            workers: add_workers.workers,
            proxy_host: add_workers.proxy_host,
            proxy_update_workers_port: add_workers.proxy_update_workers_port,
        }
    }
}
