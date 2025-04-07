use anyhow::Context;
use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::{error, json};
use thiserror::Error;

use super::api_types::{AnalyzeResultOperation, DocumentIntelligenceOperationStatus};
#[derive(Debug, Clone)]
pub struct OcrDeps {
    pub api_key: String,
    pub endpoint_url: String,
    pub model_id: String,
    pub api_version: String,
}

impl OcrDeps {
    pub fn try_from_env() -> Result<Self, anyhow::Error> {
        let azure_form_recognizer_api_key = std::env::var("AZURE_FORM_RECOGNIZER_KEY")
            .context("AZURE_FORM_RECOGNIZER_KEY env var missing")?;

        let azure_form_recognizer_endpoint_url =
            std::env::var("AZURE_FORM_RECOGNIZER_ENDPOINT_URL")
                .context("AZURE_FORM_RECOGNIZER_ENDPOINT_URL env var missing")?;

        let azure_form_recognizer_model_id = std::env::var("AZURE_FORM_RECOGNIZER_MODEL_ID")
            .context("AZURE_FORM_RECOGNIZER_MODEL_ID env var missing")?;
        let azure_form_recognizer_api_version = std::env::var("AZURE_FORM_RECOGNIZER_API_VERSION")
            .context("AZURE_FORM_RECOGNIZER_API_VERSION env var missing")?;
        Ok(OcrDeps {
            api_key: azure_form_recognizer_api_key,
            endpoint_url: azure_form_recognizer_endpoint_url,
            model_id: azure_form_recognizer_model_id,
            api_version: azure_form_recognizer_api_version,
        })
    }
}

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

    let parsed: AnalyzeResultOperation = serde_json::from_slice(&response_bytes)?;
    match parsed.status {
        DocumentIntelligenceOperationStatus::Succeeded => Ok(parsed),
        DocumentIntelligenceOperationStatus::Running
        | DocumentIntelligenceOperationStatus::Skipped
        | DocumentIntelligenceOperationStatus::NotStarted => Err(AnalysisError::InProgress),
        DocumentIntelligenceOperationStatus::Failed
        | DocumentIntelligenceOperationStatus::Canceled => Err(AnalysisError::Canceled),
    }
}
