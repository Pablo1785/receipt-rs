use std::{os::linux::raw, sync::Arc, time::Duration};

use anyhow::{anyhow, Context as _};
use axum::extract::{FromRef, Multipart, State};
use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    error::AppError,
    service::{
        ocr::{analyze_file, get_successful_analysis_results},
        process_analysis_results,
    },
};

use super::{AppDeps, DbState, OcrDeps};

#[derive(Serialize, Deserialize)]
pub struct RawResult {
    pub id: i32,
    pub result_json: String,
    pub sha256_digest: String,
}

#[derive(Clone)]
pub struct UploadDeps {
    pub client: Client,
    pub ocr: OcrDeps,
    pub pool: PgPool,
}

impl FromRef<Arc<AppDeps>> for UploadDeps {
    fn from_ref(input: &Arc<AppDeps>) -> Self {
        Self {
            client: input.client.clone(),
            ocr: input.ocr.clone(),
            pool: input.pool.clone(),
        }
    }
}

pub const WAIT_BEFORE_ASKING_FOR_RESULTS: Duration = Duration::from_secs(1);

pub async fn upload(
    State(app_state): State<UploadDeps>,
    mut multipart: Multipart,
) -> Result<axum::http::StatusCode, AppError> {
    let Some(field) = multipart.next_field().await? else {
        return Err(AppError::Anyhow(anyhow!(
            "No file was submitted for analysis"
        )));
    };
    let data = field.bytes().await?;

    let file_hash = sha256::digest(data.as_ref());

    let pool = &app_state.pool;

    let raw_result = sqlx::query_as!(
        RawResult,
        r#"SELECT * FROM raw_results WHERE sha256_digest = $1"#,
        &file_hash
    )
    .fetch_optional(pool)
    .await?;

    if let Some(raw_result) = raw_result {
        let is_already_saved = sqlx::query!(
            r#"SELECT * FROM receipts WHERE file_sha256 = $1"#,
            &raw_result.sha256_digest
        )
        .fetch_optional(pool)
        .await?.is_some();
    
        if is_already_saved {
            return Ok(axum::http::StatusCode::ACCEPTED);
        }
    } else {
        sqlx::query!(
            "INSERT INTO raw_results(result_json, sha256_digest) VALUES ($1, $2)",
            "",
            file_hash
        )
        .execute(pool)
        .await?;
        tracing::info!("Successfully cached file hash in DB. Processing further...");
    }

    let base64_file = BASE64_STANDARD.encode(data);

    tracing::info!("New file detected, starting analysis...");
    let res = analyze_file(&base64_file, &app_state.ocr, &app_state.client).await;

    if let Err(err) = res {
        tracing::error!("Error received from analysis API {:#?}", err);
        return Err(err.into());
    }
    let res = res.unwrap();
    tracing::info!("Successfully received response from analysis API. Processing...");

    let reqwest::StatusCode::ACCEPTED = res.status() else {
        return Err(AppError::Anyhow(anyhow!(
            "Analysis API responded with an error status code {}",
            res.status()
        )));
    };
    let result_url = res
        .headers()
        .get("Operation-Location")
        .ok_or(anyhow!(
            "Missing Operation-Location in response header. This should never happen"
        ))?
        .to_str()
        .with_context(|| anyhow!(
            "Could not parse Operation-Location in response header. This should never happen"
        ))?
        .to_string();
    let msg =
        format!("Successfully queued image analysis. Result will be available at: {result_url}");
    tracing::info!(msg);
    tokio::spawn(async move {
        tracing::info!("Waiting before asking for results...");
        tokio::time::sleep(WAIT_BEFORE_ASKING_FOR_RESULTS).await;
        tracing::info!("Requesting results...");

        let mut retries = 3;
        let mut wait_secs = 1;
        let process_res = loop {
            let res = get_successful_analysis_results(&result_url, &app_state.ocr, &app_state.client).await;
            tracing::info!("Received response from API. Processing...");
             match res {
                Ok(success_res) => {
                    break process_analysis_results(&file_hash, success_res, &app_state.pool).await
                }
                Err(err) => match err {
                    crate::service::ocr::AnalysisError::InProgress if retries > 0 => {
                        retries -= 1;
                        tracing::info!(
                            "Analysis is still in progress. Retrying in {} seconds...",
                            wait_secs
                        );
                        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                        wait_secs *= 2;
                    },
                    err @ _ => break Err(err.into()),
                },
            };
        };
        
        if let Err(err) = process_res {
            tracing::error!(
                "Error when processing analysis results: {}",
                err.to_string()
            );
        } else {
            tracing::info!("Successfully processed analysis results");
        }
    });
    Ok(axum::http::StatusCode::ACCEPTED)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllData {
    pub name: String,
    pub unit_price: f64,
    pub count: f64,
    pub merchant_name: String,
    pub paid_at: chrono::DateTime<chrono::Utc>,
}

pub async fn show_all(
    State(DbState { pool }): State<DbState>,
) -> Result<axum::Json<Vec<AllData>>, AppError> {
    let pool = &pool;
    let data = sqlx::query_as!(AllData, "SELECT receipts.paid_at, receipts.merchant_name, prices.count, prices.unit_price, products.name FROM receipts JOIN prices ON receipts.id = prices.receipt_id JOIN products ON products.id = prices.product_id").fetch_all(pool).await?;
    Ok(axum::Json(data))
}

pub async fn download(
    State(DbState { pool }): State<DbState>,
) -> Result<
    (
        axum::http::StatusCode,
        axum::response::AppendHeaders<[(axum::http::header::HeaderName, &'static str); 2]>,
        String,
    ),
    AppError,
> {
    let pool = &pool;
    let data = sqlx::query_as!(AllData, "SELECT receipts.paid_at, receipts.merchant_name, prices.count, prices.unit_price, products.name FROM receipts JOIN prices ON receipts.id = prices.receipt_id JOIN products ON products.id = prices.product_id").fetch_all(pool).await?;

    let content: Vec<u8> = Vec::with_capacity(data.len() * 2);
    let mut writer = csv::Writer::from_writer(content);
    for row in data {
        writer.serialize(row)?;
    }
    let content = writer.into_inner()?;

    let status = if content.len() == 0 {
        axum::http::status::StatusCode::NO_CONTENT
    } else {
        axum::http::status::StatusCode::OK
    };

    let headers: axum::response::AppendHeaders<[(axum::http::HeaderName, &str); 2]> =
        axum::response::AppendHeaders([
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"data.csv\"",
            ),
        ]);

    Ok((status, headers, String::from_utf8(content)?))
}
