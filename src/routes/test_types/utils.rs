use chrono::{DateTime, Utc};

use crate::utils::{messages::ResponseMessage, steel_extractor::TractionResult};

pub fn filter_map_tractions(
    test_results: Vec<TractionResult>,
    type_to_keep: u8,
) -> Vec<Option<TractionResult>> {
    let mut rebar_test_results = Vec::new();
    for res in test_results {
        if res.ty != type_to_keep {
            continue;
        }
        rebar_test_results.push(Some(res))
    }
    rebar_test_results
}

/// Updates the message based on whether the tractions are too few
/// or too many, then adds spacers if they are not enough.
pub fn add_traction_spacers<T>(
    traction: &mut Vec<Option<T>>,
    initial_count: usize,
    message: &mut ResponseMessage,
) {
    let new_tractions_count = traction.len();

    if new_tractions_count > initial_count {
        message.append(ResponseMessage::new_warning(format!(
            "Sono state trovate {} trazioni, ma puoi selezionarne solo {}. \
Verifica i dati del materiale prima di procedere, altrimenti le ultime {} \
trazioni verranno ignorate",
            new_tractions_count,
            initial_count,
            new_tractions_count - initial_count
        )))
    } else if new_tractions_count < initial_count {
        message.append(ResponseMessage::new_warning(format!(
            "Sono state trovate {} trazioni, ma ne servirebbero almeno {}. \
Sono stati inseriti {} spaziatori alla fine",
            new_tractions_count,
            initial_count,
            initial_count - new_tractions_count
        )));

        while traction.len() < initial_count {
            traction.push(None);
        }
    }
}

/// Obtains the machine for the given tractions.
pub fn get_test_machine<'a>(test_results: &'a [Option<TractionResult>]) -> Option<String> {
    // Get the machines for all the bars.
    test_results
        .iter()
        .map(|bar| bar.as_ref().map(|b| &b.machine))
        .filter_map(|m| m)
        .next()
        .map(|m| m.to_string())
}

pub fn get_avg_timestamp(
    test_results: &[Option<TractionResult>],
    fallback: &Option<String>,
) -> Option<String> {
    let avg_timestamp = test_results
        .iter()
        .map(|b| b.as_ref().map(|b| b.timestamp))
        .filter_map(|t| t)
        .sum::<f64>()
        / test_results.len() as f64;

    match avg_timestamp {
        x if x.is_nan() || x == 0f64 => fallback.clone(),
        x => timestamp_to_date(x),
    }
}

/// Converts a timestamp to a date, completely disregarding the nanoseconds.
fn timestamp_to_date(timestamp: f64) -> Option<String> {
    // Split into seconds + nanoseconds
    let secs = timestamp.trunc() as i64;

    let datetime = DateTime::<Utc>::from_timestamp(secs, 0)?;
    Some(datetime.format("%Y-%m-%d").to_string())
}
