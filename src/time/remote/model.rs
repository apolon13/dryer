use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NinjasResponse {
    pub datetime: String,
}