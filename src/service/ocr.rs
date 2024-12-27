use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::OcrDeps;

#[derive(Serialize, Deserialize)]
pub struct AnalyzeRequestBody {
    base64Source: String,
}


pub async fn analyze_file(
    file_string: &str,
    ocr: &OcrDeps,
    client: &Client,
) -> Result<Response, reqwest::Error> {
    let url = format!(
        "{}formrecognizer/documentModels/{}:analyze?api-version=2023-07-31", ocr.endpoint_url, ocr.model_id
    );
    let req = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header("Ocp-Apim-Subscription-Key", &ocr.api_key)
        .body(json!({ "base64Source": file_string }).to_string())
        .build()?;
    client.execute(req).await
}

pub async fn get_analysis_results(
    url: &str,
    ocr: &OcrDeps,
    client: &Client,
) -> Result<Response, reqwest::Error> {
    let req = client
        .get(url)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, "0")
        .header("Ocp-Apim-Subscription-Key", &ocr.api_key)
        .build()?;
    client.execute(req).await
}
