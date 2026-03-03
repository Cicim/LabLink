use futures::future;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    errors::{LinkError, LinkResult},
    upstream::{get_json, post_json_text_body},
};

// static MACHINE_URLS: &[&str] = &["http://192.168.20.250:80", "http://192.168.20.251:80"];
static MACHINE_URLS: &[&str] = &["http://192.168.20.250:80"];

pub(crate) async fn get_test_results(
    client: &Client,
    request_name: &str,
) -> LinkResult<Vec<TestResult>> {
    // Run the two requests concurrently.
    let results = future::join_all(
        MACHINE_URLS
            .into_iter()
            .map(|machine_url| read_machine(client, request_name, machine_url)),
    )
    .await;

    let mut final_vec = Vec::new();

    // Join all the requests that didn't fail.
    for result in results {
        let result = result?;
        final_vec.extend(result);
    }

    Ok(final_vec)
}

#[derive(Deserialize)]
struct SuccessfulReadResult {
    va: String,
    results: Vec<TestResult>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TestResult {
    pub id: String,
    pub diameter: f32,
    side_a: f32,
    side_b: f32,
    pub mass: f32,
    pub length: f32,
    pub fy: f32,
    pub ft: f32,
    pub f02: bool,
    #[serde(rename = "type")]
    pub ty: u8,
    pub timestamp: f64,
    #[serde(default = "controls")]
    pub machine: String,
}

fn controls() -> String {
    "controls".into()
}

/// Reads all the results of the traction related to a request_name from a given machine serving
/// data through the protocol defined in https://github.com/Cicim/steel-controls-extractor
async fn read_machine(
    client: &Client,
    request_name: &str,
    machine_url: &str,
) -> LinkResult<Vec<TestResult>> {
    if !test_connection(client, machine_url).await {
        return Ok(Vec::new());
    }

    let response: LinkResult<SuccessfulReadResult> =
        post_json_text_body(client, machine_url, request_name.to_string()).await;

    match response {
        Ok(SuccessfulReadResult { va, results }) => {
            tracing::trace!("Found {} steel tests for {va}", results.len());
            Ok(results)
        }
        // If it returns a non-success error-code
        Err(LinkError::UpstreamStatus { .. }) => Ok(Vec::new()),
        // Otherwise propagate the error
        Err(x) => Err(x),
    }
}

#[derive(Deserialize)]
struct HealthMessage {
    status: String,
}

/// Checks connection to a given steel traction machine.
async fn test_connection(client: &Client, machine_url: &str) -> bool {
    // Connect to the address at the root to receive the health status.
    match get_json::<HealthMessage>(client, machine_url).await {
        Ok(HealthMessage { status }) => status == "ok",
        Err(e) => {
            tracing::error!("Error while testing connection to {}: {}", machine_url, e);
            false
        }
    }
}
