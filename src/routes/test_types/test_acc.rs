use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::LinkResult;
use crate::handle_test;
use crate::routes::test_types::utils::*;
use crate::upstream::build_client;
use crate::utils::steel_extractor::*;
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
        let machine = get_test_machine(&bars);

        let mut groups = vec![];

        for group in self.groups.iter() {
            let group_size = group.samples.len();
            // Take the next n bars from that group
            let group_bars = bars.split_at(group_size).0;
            let date = get_avg_timestamp(&group_bars, &group.date);

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
                date,
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

impl From<TestResult> for RebarTestResult {
    fn from(value: TestResult) -> Self {
        Self {
            id: value.id.clone(),
            diameter: value.diameter,
            mass: value.mass,
            length: value.length,
            fy: (value.fy * 1000f32).trunc() / 1000f32,
            ft: (value.ft * 1000f32).trunc() / 1000f32,
            f02: value.f02,
            timestamp: value.timestamp.round(),
            machine: value.machine.clone(),
        }
    }
}

impl CommonProperties for RebarTestResult {
    fn get_timestamp(&self) -> f64 {
        self.timestamp
    }
    fn get_machine(&self) -> &str {
        &self.machine
    }
}

/// /api/tests/ACC/get route
async fn read_from_machine_handler(
    Json(input): Json<RequestIdAndTestData<Minute>>,
) -> LinkResult<Json<FrontendDialogData<RebarTestResult>>> {
    let client = build_client()?;
    let (mut test_results, mut message) = get_test_results(&client, &input.request_id).await?;

    // Sort the test_results by diameter, then timestamp.
    test_results.sort_by(|a, b| {
        a.diameter
            .partial_cmp(&b.diameter)
            .unwrap()
            .then(a.timestamp.partial_cmp(&b.timestamp).unwrap())
    });

    let mut rebar_test_results = filter_map_tractions(test_results, 0);
    let initial_bar_count = input.test_data.count_bars();
    add_traction_spacers(&mut rebar_test_results, initial_bar_count, &mut message);

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

pub(super) fn router() -> Router {
    Router::new()
        .route("/", get(test_source_handler))
        .route("/get", post(read_from_machine_handler))
        .route("/get/callback", post(callback_handler))
}
