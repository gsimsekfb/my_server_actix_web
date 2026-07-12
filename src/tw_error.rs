use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: String,
    pub field: Option<&'static str>,
}

