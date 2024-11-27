use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(clap::ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Add,
    Remove,
}

impl Action {
    pub fn as_str(&self) -> &str {
        match self {
            Action::Add => "add",
            Action::Remove => "remove",
        }
    }
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct UpdateWorkers {
    pub action: Action,
    pub workers: Vec<String>,
}

impl UpdateWorkers {
    pub fn execute(&self) -> Result<(), String> {
        // Define a runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {:?}", e))?;

        let query_params = serde_qs::to_string(&self).map_err(|err| err.to_string())?;

        println!("Action: {:?}, with workers: {:?}", self.action, self.workers);

        let url = format!("http://0.0.0.0:8082?{}", query_params);

        // Create an HTTP/2 client
        let client = Client::builder()
            .http2_prior_knowledge()
            .build()
            .map_err(|err| err.to_string())?;

        // Make the request
        let response = rt.block_on(client.get(url).send()).map_err(|err| err.to_string())?;

        // Check status code
        if !response.status().is_success() {
            return Err(format!("Request failed with status code: {}", response.status()));
        }

        // Read the X-Workers-Amount header
        let workers_amount = response
            .headers()
            .get("X-Workers-Amount")
            .ok_or("Missing X-Workers-Amount header")?
            .to_str()
            .map_err(|err| err.to_string())?;

        println!("New amount of workers: {}", workers_amount);

        Ok(())
    }
}
