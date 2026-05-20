use std::collections::HashMap;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::LinkResult;
use crate::handle_test;
use crate::routes::test_types::utils::*;
use crate::upstream::build_client;
use crate::utils::messages::ResponseMessage;
use crate::utils::steel_extractor::*;
use crate::utils::{
    FrontendDialogData, Method, MethodInput, MethodOutput, RequestIdAndTestData, TestDataAndRows,
};

/// The minute type in the beginning.
#[derive(Debug, Serialize, Deserialize)]
struct Minute {
    /// The single machine used by all samples in a minute
    ///
    /// Remove in favor of [`Group::machine`]
    macchina: Option<String>,
    groups: Vec<Group>,
}

impl Minute {
    /// Count the number of bars in the minute.
    fn count_bars(&self) -> usize {
        self.groups.iter().map(|g| g.samples.iter().count()).sum()
    }

    /// Gets the original labels in the minute.
    fn get_traction_labels(&self) -> Vec<String> {
        self.groups
            .iter()
            .flat_map(|g| {
                g.samples.iter().map(|s| {
                    format!(
                        "Ø{}/{} {}",
                        s.diam,
                        s.n.as_ref().unwrap_or(&0.0),
                        s.c.as_deref().unwrap_or("?")
                    )
                })
            })
            .collect()
    }

    /// Get the diameters in the minute, which determine the
    /// order in which the samples are loaded.
    fn get_numbered_diameters(&self) -> Vec<(u8, u16)> {
        let mut diameter_with_nums = Vec::new();
        let mut last_diameter_num = HashMap::new();

        for group in self.groups.iter() {
            for sample in group.samples.iter() {
                let diameter = sample.diam as u8;
                if !last_diameter_num.contains_key(&diameter) {
                    last_diameter_num.insert(diameter, 1);
                    diameter_with_nums.push((diameter, 1u16));
                } else {
                    let next_num = last_diameter_num[&diameter] + 1;
                    last_diameter_num.insert(diameter, next_num);
                    diameter_with_nums.push((diameter, next_num as u16));
                }
            }
        }

        diameter_with_nums
    }

    fn rebuild_with_bars(&self, mut bars: Vec<Option<TractionResult>>) -> Minute {
        let machine = get_test_machine(&bars);

        let mut groups = vec![];

        for group in self.groups.iter() {
            let group_size = group.samples.len();
            // Take the next n bars from that group
            let group_bars = bars.split_at(group_size).0;
            let date = get_avg_timestamp(&group_bars, &group.date);
            let machine = get_test_machine(&group_bars);

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
                        // Since these are computed, we don't need to provide values.
                        diam: 0.0,
                        n: None,
                        c: None,
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
                machine: machine.or(group.machine.clone()),
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
    machine: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Sample {
    n: Option<f32>,
    c: Option<String>,
    /// This is computed, and we only need it to sort the samples by diameter.
    diam: f32,

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

/// /api/tests/ACC/get route
async fn read_from_machine_handler(
    Json(input): Json<RequestIdAndTestData<Minute>>,
) -> LinkResult<Json<FrontendDialogData<TractionResult>>> {
    let client = build_client()?;
    let (mut test_results, mut message) = get_test_results(&client, &input.request_id).await?;

    // Order the test_results by timestamp.
    test_results.sort_by(|a, b| {
        a.timestamp
            .partial_cmp(&b.timestamp)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut labels = input.test_data.get_traction_labels();

    // Filter only the ones with type 0 (rebar)
    let mut rebar_test_results = Vec::new();
    for res in test_results {
        if res.ty != 0 {
            continue;
        }
        rebar_test_results.push(res)
    }

    let no_results = rebar_test_results.is_empty();
    if no_results {
        message.append(ResponseMessage::new_error(
            "Non è stata trovata nessuna barra tra i risultati".to_string(),
        ));
    }

    // Sort the results by how the diameters appear in the minute.
    let minute_diameters = input.test_data.get_numbered_diameters();
    let mut ordered_test_results = Vec::new();

    for (diameter, number) in minute_diameters {
        // Find the first diameter in the results and add it to the ordered list.
        let next_bar_index = rebar_test_results
            .iter()
            .position(|r| r.diameter as u8 == diameter || r.diameter as u8 + 1 == diameter);

        match next_bar_index {
            Some(index) => {
                // Remove the bar from the original list and add it to the ordered list.
                let mut bar = rebar_test_results.remove(index);

                if bar.diameter as u8 + 1 == diameter {
                    message.append(ResponseMessage::new_warning(format!(
                        "La barra {}/{} è stata trovata (forse), ma con diametro {}",
                        diameter, number, bar.diameter
                    )));
                    bar.diameter = diameter as f32;
                }

                ordered_test_results.push(Some(bar));
            }
            None => {
                if !no_results {
                    // Notify the user that the bar was not found for this diameter.
                    message.append(ResponseMessage::new_warning(format!(
                        "La barra {}/{} non è stata trovata",
                        diameter, number
                    )));
                }
                ordered_test_results.push(None);
            }
        }
    }

    // Add any remaining bars to the ordered list, in the order they appear in the minute.
    if !rebar_test_results.is_empty() {
        message.append(ResponseMessage::new_error(format!(
            "Ci sono {} barre non associate a quelle nella minuta; le troverai in fondo, con etichetta ---",
            rebar_test_results.len()
        )));
        for bar in rebar_test_results {
            ordered_test_results.push(Some(bar));
            labels.push("---".into());
        }
    }

    Ok(Json(FrontendDialogData {
        rows: ordered_test_results,
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
        labels,
    }))
}

/// /api/tests/ACC/get/callback route
async fn callback_handler(
    Json(TestDataAndRows {
        test_data,
        mut rows,
    }): Json<TestDataAndRows<Minute, TractionResult>>,
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
