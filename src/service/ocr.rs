use anyhow::Context;
use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::{error, json};
use thiserror::Error;

use crate::{
    http::OcrDeps,
    manual::{AnalyzeResultOperation, DocumentIntelligenceOperationStatus},
};

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
        "{}documentintelligence/documentModels/{}:analyze?api-version={}",
        ocr.endpoint_url, ocr.model_id, ocr.api_version
    );
    let req = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header("Ocp-Apim-Subscription-Key", &ocr.api_key)
        .body(json!({ "base64Source": file_string }).to_string())
        .build()?;
    client.execute(req).await
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum AnalysisError {
    #[error("Analysis is still in progress")]
    InProgress,
    #[error("Analysis canceled")]
    Canceled,
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error(transparent)]
    ResponseParsing(#[from] serde_json::Error),
}

pub async fn get_successful_analysis_results(
    url: &str,
    ocr: &OcrDeps,
    client: &Client,
) -> Result<AnalyzeResultOperation, AnalysisError> {
    let req = client
        .get(url)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, "0")
        .header("Ocp-Apim-Subscription-Key", &ocr.api_key)
        .build()?;
    let res = client.execute(req).await?;
    let response_bytes = res.bytes().await?;

    let response_text = String::from_utf8_lossy(&response_bytes);
    tracing::info!("Received response from API: {}", response_text);
    let parsed: AnalyzeResultOperation = serde_json::from_slice(&response_bytes)?;
    match parsed.status {
        DocumentIntelligenceOperationStatus::Succeeded => Ok(parsed),
        DocumentIntelligenceOperationStatus::Running
        | DocumentIntelligenceOperationStatus::Skipped
        | DocumentIntelligenceOperationStatus::NotStarted => {
            Err(AnalysisError::InProgress)
        }
        DocumentIntelligenceOperationStatus::Failed
        | DocumentIntelligenceOperationStatus::Canceled => Err(AnalysisError::Canceled),
    }
}
