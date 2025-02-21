use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: Option<i64>,
    pub message: Option<String>,
    pub err_msg: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "code({}) {}",
            self.code.unwrap_or(-1),
            self.message
                .as_ref()
                .or(self.err_msg.as_ref())
                .map(String::as_str)
                .unwrap_or("")
        )
    }
}

impl std::error::Error for ApiError {}
