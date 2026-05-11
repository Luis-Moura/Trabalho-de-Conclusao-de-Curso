use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ShortenRequest {
    pub url: String,
}

#[derive(Serialize)]
pub struct ShortenResponse {
    #[serde(rename = "shortUrl")]
    pub short_url: String,
}
