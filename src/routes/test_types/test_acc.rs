use axum::routing::{get, post};
use axum::{extract::State, Json, Router};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::errors::LinkResult;
use crate::handle_test;
use crate::utils::messages::ResponseMessage;
use crate::utils::steel_extractor::get_test_results;
use crate::utils::{
    FrontendDialogData, Method, MethodInput, MethodOutput, RequestIdAndTestData, TestDataAndRows,
};

/// The minute type in the beginning.
#[derive(Debug, Serialize, Deserialize)]
struct Minute {
    macchina: Option<String>,
    groups: Vec<Group>,
}

impl Minute {
    /// Count the number of bars in the minute.
    fn count_bars(&self) -> usize {
        self.groups.iter().map(|g| g.samples.iter().count()).sum()
    }

    fn rebuild_with_bars(&self, mut bars: Vec<Option<RebarTestResult>>) -> Minute {
        // Get the machines for all the bars.
        let machine = bars
            .iter()
            .map(|bar| bar.as_ref().map(|b| b.machine.as_str()))
            .filter_map(|m| m)
            .next()
            .map(|m| m.to_string());

        let mut groups = vec![];

        for group in self.groups.iter() {
            let group_size = group.samples.len();
            // Take the next n bars from that group
            let group_bars = bars.split_at(group_size).0;

            // Compute the timestamp for this group.
            let avg_timestamp = group_bars
                .iter()
                .map(|b| b.as_ref().map(|b| b.timestamp))
                .filter_map(|t| t)
                .sum::<f64>()
                / group_bars.len() as f64;

            let mut samples = vec![];

            for i in 0..group_size {
                let bar = bars.remove(0);

                let sample = if let Some(bar) = bar {
                    Sample {
                        f02: bar.f02,
                        ft: Some(bar.ft),
                        fy: Some(bar.fy),
                        length: Some(bar.length),
                        mass: Some(bar.mass),
                        lu: group.samples[i].lu.clone(),
                        pieg: group.samples[i].pieg.clone(),
                    }
                } else {
                    // Keep the old sample.
                    group.samples[i].clone()
                };

                samples.push(sample)
            }

            groups.push(Group {
                date: match avg_timestamp {
                    x if x.is_nan() || x == 0f64 => group.date.clone(),
                    x => Some(timestamp_to_date(x)),
                },
                sn: group.sn,
                steelworks: group.steelworks,
                format: group.format.clone(),
                samples,
            })
        }

        Minute {
            macchina: machine,
            groups,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Group {
    sn: Option<u16>,
    date: Option<String>,
    samples: Vec<Sample>,
    steelworks: Option<u16>,
    format: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Sample {
    // n: computed
    // c: computed
    // diam: computed
    mass: Option<f32>,
    length: Option<f32>,
    // deff: computed,
    // aeff: computed,
    fy: Option<f32>,
    f02: bool,
    ft: Option<f32>,
    // ft_fy: computed
    // fy_fynom: computed
    lu: Option<f32>,
    // agt: computed
    // dmand: related to bending
    pieg: Option<String>,
}

/// Converts a timestamp to a date, completely disregarding the nanoseconds.
fn timestamp_to_date(timestamp: f64) -> String {
    // Split into seconds + nanoseconds
    let secs = timestamp.trunc() as i64;

    let datetime = DateTime::<Utc>::from_timestamp(secs, 0).expect("Invalid timestamp");
    datetime.format("%Y-%m-%d").to_string()
}

#[derive(Clone)]
pub struct LinkState {
    pub client: Client,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RebarTestResult {
    id: String,
    diameter: f32,
    mass: f32,
    length: f32,
    fy: f32,
    ft: f32,
    f02: bool,
    timestamp: f64,
    machine: String,
}

/// /api/tests/ACC/get route
async fn read_from_machine_handler(
    State(state): State<LinkState>,
    Json(input): Json<RequestIdAndTestData<Minute>>,
) -> LinkResult<Json<FrontendDialogData<RebarTestResult>>> {
    let (mut test_results, mut message) =
        get_test_results(&state.client, &input.request_id).await?;

    // Sort the test_results by diameter, then timestamp.
    test_results.sort_by(|a, b| {
        a.diameter
            .partial_cmp(&b.diameter)
            .unwrap()
            .then(a.timestamp.partial_cmp(&b.timestamp).unwrap())
    });

    let mut rebar_test_results = Vec::new();

    let initial_bar_count = input.test_data.count_bars();

    for res in test_results {
        if res.ty != 0 {
            continue;
        }
        rebar_test_results.push(Some(RebarTestResult {
            id: res.id.clone(),
            diameter: res.diameter,
            mass: res.mass,
            length: res.length,
            fy: (res.fy * 1000f32).trunc() / 1000f32,
            ft: (res.ft * 1000f32).trunc() / 1000f32,
            f02: res.f02,
            timestamp: res.timestamp.round(),
            machine: res.machine.clone(),
        }))
    }
    let new_bar_count = rebar_test_results.len();

    if new_bar_count > initial_bar_count {
        message = message.join(ResponseMessage::new_warning(format!(
            "Sono state trovate {} barre, ma puoi selezionarne solo {}. \
Verifica i dati del materiale prima di procedere, altrimenti le ultime {} \
barre verranno ignorate",
            new_bar_count,
            initial_bar_count,
            new_bar_count - initial_bar_count
        )))
    } else if new_bar_count < initial_bar_count {
        message = message.join(ResponseMessage::new_warning(format!(
            "Sono state trovate {} barre, ma ne servirebbero almeno {}. \
Sono stati inseriti {} spaziatori alla fine",
            new_bar_count,
            initial_bar_count,
            initial_bar_count - new_bar_count
        )));

        while rebar_test_results.len() < initial_bar_count {
            rebar_test_results.push(None);
        }
    }

    Ok(Json(FrontendDialogData {
        rows: rebar_test_results,
        column_names: vec![
            ("id", "ID"),
            ("diameter", "Diametro"),
            ("mass", "Massa"),
            ("length", "Lunghezza"),
            ("fy", "Fy"),
            ("ft", "Ft"),
            ("f02", "F02"),
            ("timestamp", "Timestamp"),
            ("machine", "Macchina"),
        ],
        message,
    }))
}

/// /api/tests/ACC/get/callback route
async fn callback_handler(
    State(_): State<LinkState>,
    Json(TestDataAndRows {
        test_data,
        mut rows,
    }): Json<TestDataAndRows<Minute, RebarTestResult>>,
) -> LinkResult<Json<Minute>> {
    let minute_bars = test_data.count_bars();
    // Remove all the extra bars from the input.
    while rows.len() > minute_bars {
        rows.pop();
    }
    while rows.len() < minute_bars {
        rows.push(None);
    }

    // Fill the minute with the bars.
    let test_data = test_data.rebuild_with_bars(rows);

    Ok(Json(test_data))
}

handle_test!(
    "ACC.TP",
    vec![Method::new("get", "Leggi i dati dalla macchina")
        .with_input(MethodInput::RequestNameAndTestData)
        .with_output(MethodOutput::NewTestDataAfterSelection {
            callback: "get/callback".to_string()
        }),]
);

pub fn router(client: Client) -> Router {
    let state = LinkState { client };
    Router::new()
        .route("/ACC.TP", get(test_source_handler))
        .route("/ACC.TP/get", post(read_from_machine_handler))
        .route("/ACC.TP/get/callback", post(callback_handler))
        .with_state(state)
}
