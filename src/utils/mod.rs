use messages::ResponseMessage;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub(crate) mod machines;
pub(crate) mod messages;
pub(crate) mod steel_extractor;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub(crate) struct RequestIdAndTestData<T> {
    pub request_id: String,
    pub test_data: T,
}

/// The information for the LabLink dialog input to select rows.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "R: Serialize + DeserializeOwned")]
pub(crate) struct FrontendDialogData<R> {
    pub(crate) rows: Vec<Option<R>>,
    pub(crate) column_names: Vec<(&'static str, &'static str)>,
    pub(crate) message: ResponseMessage,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound = "T: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned")]
pub(crate) struct TestDataAndRows<T, R> {
    pub test_data: T,
    pub rows: Vec<Option<R>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MethodList {
    test_name: String,
    functions: Vec<Method>,
}

impl MethodList {
    pub(crate) fn new(test_name: &str, functions: Vec<Method>) -> Self {
        MethodList {
            test_name: test_name.to_string(),
            functions,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct Method {
    path: String,
    name: String,
    input: Option<MethodInput>,
    output: Option<MethodOutput>,
}

impl Method {
    pub(crate) fn new(path: &str, name: &str) -> Self {
        Method {
            path: path.to_string(),
            name: name.to_string(),
            input: None,
            output: None,
        }
    }

    pub(crate) fn with_input(mut self, input: MethodInput) -> Self {
        self.input = Some(input);
        self
    }

    pub(crate) fn with_output(mut self, output: MethodOutput) -> Self {
        self.output = Some(output);
        self
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum MethodInput {
    /// Receives the request id and the test data.
    #[serde(rename = "search-request")]
    RequestNameAndTestData,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub(crate) enum MethodOutput {
    /// Returns a table that can be manipulated by the client,
    /// but which needs to be sent back with the test data in
    /// order to build the final result.
    #[serde(rename = "search-response")]
    NewTestDataAfterSelection { callback: String },
}

#[macro_export]
macro_rules! handle_test {
    ($code:expr, $functions:expr) => {
        async fn test_source_handler(
        ) -> crate::errors::LinkResult<axum::Json<crate::utils::MethodList>> {
            Ok(axum::Json(crate::utils::MethodList::new($code, $functions)))
        }
    };
}
