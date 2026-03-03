use futures::future;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    errors::{LinkError, LinkResult},
    upstream::{get_json, post_json_text_body},
};

use super::{
    machines::{Machine, MACHINES},
    messages::ResponseMessage,
};

static STEEL_MACHINES: &[&Machine] = &[&MACHINES[0], &MACHINES[1]];

pub(crate) async fn get_test_results(
    client: &Client,
    request_name: &str,
) -> LinkResult<(Vec<TestResult>, ResponseMessage)> {
    // Run the two requests concurrently.
    let results = future::join_all(
        STEEL_MACHINES
            .into_iter()
            .map(|machine| read_machine(client, request_name, machine)),
    )
    .await;

    let mut final_vec = Vec::new();
    let mut final_message: Option<ResponseMessage> = None;

    // Join all the requests that didn't fail.
    for result in results {
        let (result, new_message) = result?;
        final_vec.extend(result);
        final_message = match final_message {
            None => Some(new_message),
            Some(message) => Some(message.join(new_message)),
        }
    }

    Ok((final_vec, final_message.unwrap()))
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
    // side_a: f32,
    // side_b: f32,
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
    machine: &Machine,
) -> LinkResult<(Vec<TestResult>, ResponseMessage)> {
    if !test_connection(client, machine).await {
        return Ok((
            Vec::new(),
            ResponseMessage::new_error(format!(
                "Impossibile connettersi a {}. Potrebbero mancare dei risultati",
                machine.name
            )),
        ));
    }

    let response: LinkResult<SuccessfulReadResult> =
        post_json_text_body(client, machine.url, request_name.to_string()).await;

    match response {
        Ok(SuccessfulReadResult { va, results }) => {
            tracing::trace!("Found {} steel tests for {va}", results.len());

            let results_len = results.len();
            Ok((
                results,
                ResponseMessage::new_info(format!(
                    "Trovati {} risultati su {}",
                    results_len, machine.name
                )),
            ))
        }
        // If it returns a non-success error-code
        Err(LinkError::UpstreamStatus { .. }) => Ok((
            Vec::new(),
            ResponseMessage::new_info(format!("Nessun risultato trovato su {}", machine.name)),
        )),
        // Otherwise propagate the error
        Err(x) => Err(x),
    }
}

#[derive(Deserialize)]
struct HealthMessage {
    status: String,
}

/// Checks connection to a given steel traction machine.
async fn test_connection(client: &Client, machine: &Machine) -> bool {
    // Connect to the address at the root to receive the health status.
    match get_json::<HealthMessage>(client, machine.url).await {
        Ok(HealthMessage { status }) => status == "ok",
        Err(e) => {
            tracing::error!("Error while testing connection to {}: {}", machine.url, e);
            false
        }
    }
}
