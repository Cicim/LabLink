use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ResponseMessage {
    message: String,
    severity: Severity,
}

#[derive(Serialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
pub(crate) enum Severity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

impl ResponseMessage {
    pub fn new<S: Into<String>>(message: S, severity: Severity) -> Self {
        ResponseMessage {
            message: message.into(),
            severity,
        }
    }

    pub fn new_info<S: Into<String>>(message: S) -> Self {
        ResponseMessage::new(message, Severity::Info)
    }

    pub fn new_warning<S: Into<String>>(message: S) -> Self {
        ResponseMessage::new(message, Severity::Warning)
    }

    pub fn new_error<S: Into<String>>(message: S) -> Self {
        ResponseMessage::new(message, Severity::Error)
    }

    /// Joins two messages together, keeping the higher severity.
    pub fn join(self, other: Self) -> Self {
        ResponseMessage {
            message: format!("{}. {}", self.message, other.message),
            severity: self.severity.max(other.severity),
        }
    }

    /// Appends a message to this one, keeping the higher severity.
    pub fn append(&mut self, other: Self) {
        self.message = format!("{}. {}", self.message, other.message);
        self.severity = self.severity.max(other.severity);
    }
}
