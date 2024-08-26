use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use axum::extract::{FromRef, Multipart, State};
use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    service::{
        ocr::{analyze_file, get_analysis_results},
        process_analysis_results,
    },
    AppError,
};

use super::{AppDeps, DbState};

#[derive(Serialize, Deserialize)]
pub struct RawResult {
    pub id: i32,
    pub result_json: String,
    pub sha256_digest: String,
}

#[derive(Clone)]
pub struct UploadDeps {
    pub client: Client,
    pub azure_form_recognizer_api_key: String,
    pub pool: PgPool,
}

impl FromRef<Arc<AppDeps>> for UploadDeps {
    fn from_ref(input: &Arc<AppDeps>) -> Self {
        Self {
            client: input.client.clone(),
            azure_form_recognizer_api_key: input.azure_form_recognizer_api_key.clone(),
            pool: input.pool.clone(),
        }
    }
}

pub async fn upload(
    State(app_state): State<UploadDeps>,
    mut multipart: Multipart,
) -> Result<String, AppError> {
    if let Some(field) = multipart.next_field().await? {
        let data = field.bytes().await?;

        let file_hash = sha256::digest(data.as_ref());

        let pool = &app_state.pool;

        let is_already_analyzed = sqlx::query!(
            "SELECT * FROM raw_results WHERE sha256_digest = $1",
            &file_hash
        )
        .fetch_optional(pool)
        .await?
        .is_some();

        if is_already_analyzed {
            return Err(AppError::Anyhow(anyhow!(
                "Submitted file's hash is already saved in the DB. Not runnning analysis."
            )));
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
        let res = analyze_file(
            &base64_file,
            &app_state.azure_form_recognizer_api_key,
            &app_state.client,
        )
        .await?;
        tracing::info!("Successfully received response from analysis API. Processing...");

        if let reqwest::StatusCode::ACCEPTED = res.status() {
            let result_url = res
                .headers()
                .get("Operation-Location")
                .ok_or(anyhow!(
                    "Missing Operation-Location in response header. This should never happen"
                ))?
                .to_str()?
                .to_string();
            let msg = format!(
                "Successfully queued image analysis. Result will be available at: {result_url}"
            );
            tracing::info!(msg);
            tokio::spawn(async move {
                tracing::info!("Waiting before asking for results...");
                tokio::time::sleep(Duration::from_secs(30)).await;
                tracing::info!("Requesting results...");
                let res = get_analysis_results(
                    &result_url,
                    &app_state.azure_form_recognizer_api_key,
                    &app_state.client,
                )
                .await;
                tracing::info!("Received response from API. Processing...");
                let process_res = match res {
                    Ok(success_res) => {
                        process_analysis_results(&file_hash, success_res, &app_state.pool).await
                    }
                    Err(err) => Err(err.into()),
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
            Ok(msg)
        } else {
            Err(AppError::Anyhow(anyhow!(
                "Analysis API responded with an error status code {}",
                res.status()
            )))
        }
    } else {
        Err(AppError::Anyhow(anyhow!(
            "No file was submitted for analysis"
        )))
    }
}

#[derive(Serialize, Deserialize)]
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

    let headers: axum::response::AppendHeaders<[(axum::http::HeaderName, &str); 2]> =
        axum::response::AppendHeaders([
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"data.csv\"",
            ),
        ]);

    Ok((headers, String::from_utf8(content)?))
}
