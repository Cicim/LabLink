use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::LinkResult;
use crate::handle_test;
use crate::routes::test_types::utils::*;
use crate::upstream::build_client;
use crate::utils::{steel_extractor::*, TestDataAndRows};
use crate::utils::{FrontendDialogData, Method, MethodInput, MethodOutput, RequestIdAndTestData};

/// Root payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Minute {
    // computed
    // pub start_date: Option<NaiveDate>,
    // pub end_date: Option<NaiveDate>,
    #[serde(default)]
    pub samples: Vec<Sample>,
}

impl Minute {
    fn count_tractions(&self) -> usize {
        self.samples.iter().map(|s| s.tr.len()).sum()
    }

    fn rebuild_with_tractions(self, mut new_tractions: Vec<Option<TractionResult>>) -> Self {
        let mut samples = Vec::new();

        for sample in self.samples {
            let num_tr = sample.tr.len();
            let group_bars = new_tractions.split_at(num_tr).0;
            let date = get_avg_timestamp(group_bars, &sample.data_tr);

            let mut tr = Vec::new();
            for i in 0..num_tr {
                tr.push(if let Some(res) = new_tractions.remove(0) {
                    TractionTest {
                        massa: sample.tr[i].massa.clone(),
                        l: sample.tr[i].l.clone(),
                        s: Some(res.side_a),
                        b: Some(res.side_b),
                        f02: res.f02,
                        fy: Some(res.fy),
                        ft: Some(res.ft),
                        lu: sample.tr[i].lu.clone(),
                    }
                } else {
                    // Keep the old sample.
                    sample.tr[i].clone()
                });
            }

            samples.push(Sample {
                data_tr: date,
                tr,
                ..sample
            })
        }

        Self { samples }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub data_tr: Option<String>,

    pub bda: Option<String>,
    #[serde(default)]
    pub rid: bool,

    #[serde(default)]
    pub tr: Vec<TractionTest>,

    // Important to carry along, but the implementation detail is not relevant.
    #[serde(default)]
    pub set_res: Value,
    #[serde(default)]
    pub res: Value,
    // computed
    // pub sigla: Option<String>,
    // pub tipo: Option<String>,
    // pub qual: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TractionTest {
    pub massa: Option<f32>,
    pub l: Option<f32>,
    pub s: Option<f32>,
    pub b: Option<f32>,

    #[serde(default)]
    pub f02: bool,

    pub fy: Option<f32>,
    pub ft: Option<f32>,
    pub lu: Option<f32>,
    // computed
    // pub n: Option<u32>,
    // pub s0: Option<f64>,
    // pub l0: Option<f64>,
    // pub bda: Option<f64>,
    // pub a: Option<f64>,
}

async fn read_from_machine_handler(
    Json(input): Json<RequestIdAndTestData<Minute>>,
) -> LinkResult<Json<FrontendDialogData<TractionResult>>> {
    let client = build_client()?;
    let (all_results, mut message) = get_test_results(&client, &input.request_id).await?;

    let mut traction_results = filter_map_tractions(all_results, 1);
    let initial_tractions_count = input.test_data.count_tractions();
    add_traction_spacers(&mut traction_results, initial_tractions_count, &mut message);

    Ok(Json(FrontendDialogData {
        rows: traction_results,
        column_names: vec![
            ("profile", "Profilo"),
            ("id", "ID"),
            ("quality", "Qualità"),
            ("s", "Spessore"),
            ("b", "Larghezza"),
            ("f02", "F02"),
            ("fy", "Fy"),
            ("ft", "Ft"),
            ("timestamp", "Timestamp"),
            ("machine", "Macchina"),
        ],
        message,
    }))
}

async fn callback_handler(
    Json(TestDataAndRows {
        test_data,
        mut rows,
    }): Json<TestDataAndRows<Minute, TractionResult>>,
) -> LinkResult<Json<Minute>> {
    let minute_tractions = test_data.count_tractions();
    // Remove all the extra tractions from the input.
    while rows.len() > minute_tractions {
        rows.pop();
    }
    while rows.len() < minute_tractions {
        rows.push(None);
    }

    // Fill the minute with the bars.
    let test_data = test_data.rebuild_with_tractions(rows);

    Ok(Json(test_data))
}

handle_test!(
    "CARP.LP",
    Method::new("get", "Leggi i dati dalla macchina")
        .with_input(MethodInput::RequestNameAndTestData)
        .with_output(MethodOutput::NewTestDataAfterSelection {
            callback: "get/callback".to_string()
        })
);

pub(super) fn router() -> Router {
    Router::new()
        .route("/", get(test_source_handler))
        .route("/get", post(read_from_machine_handler))
        .route("/get/callback", post(callback_handler))
}
