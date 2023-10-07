use chrono::{FixedOffset, TimeZone};
use shuttle_persist::{PersistError, PersistInstance};
use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use axum::{
    extract::{multipart::MultipartError, DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use base64::{prelude::BASE64_STANDARD, Engine};

use chrono_tz::Europe::Copenhagen;
use manual::AnalyzeResultOperation;
use reqwest::{
    header::{ToStrError, CONTENT_LENGTH, CONTENT_TYPE},
    Client, Response, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shuttle_axum::ShuttleAxum;
use shuttle_runtime::{self};
use shuttle_secrets::SecretStore;
use sqlx::{Executor, PgPool, Row};
use thiserror::Error;

mod manual;

#[derive(Serialize, Deserialize)]
struct AnalyzeRequestBody {
    base64Source: String,
}

const ENDPOINT: &str = "https://receipt-model.cognitiveservices.azure.com/";
const MODEL_ID: &str = "prebuilt-receipt";

async fn analyze_file(
    file_string: &str,
    api_key: &str,
    client: &Client,
) -> Result<Response, reqwest::Error> {
    let url = format!(
        "{ENDPOINT}formrecognizer/documentModels/{MODEL_ID}:analyze?api-version=2023-07-31"
    );
    let req = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .body(json!({ "base64Source": file_string }).to_string())
        .build()?;
    client.execute(req).await
}

async fn get_analysis_results(
    url: &str,
    api_key: &str,
    client: &Client,
) -> Result<Response, reqwest::Error> {
    let req = client
        .get(url)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, "0")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .build()?;
    client.execute(req).await
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Error, Debug)]
#[error(transparent)]
enum AppError {
    #[error(transparent)]
    Multipart(#[from] MultipartError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    ToStr(#[from] ToStrError),
    #[error(transparent)]
    EncodeSlice(#[from] base64::EncodeSliceError),
    #[error(transparent)]
    HttpClient(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    #[error(transparent)]
    ShuttlePersist(#[from] PersistError),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.to_string()),
        )
            .into_response()
    }
}

async fn process_analysis_results(
    file_hash: &str,
    res: reqwest::Response,
    app_state: Arc<AppState>,
) -> Result<(), AppError> {
    let text = res.text().await?;
    app_state.persist.save(file_hash, &text)?;
    tracing::info!("Successfully cached raw response text in KV storage. Processing further...");
    let data: manual::AnalyzeResultOperation = serde_json::from_str(&text)?;
    save_analysis_data(&app_state.pool, data).await?;
    tracing::info!("Successfully saved receipt data in database");
    Ok::<(), AppError>(())
}

async fn upload(
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<String, AppError> {
    if let Some(field) = multipart.next_field().await? {
        let data = field.bytes().await?;

        let file_hash = sha256::digest(data.as_ref());

        let is_already_analyzed = app_state
            .persist
            .list()?
            .into_iter()
            .find(|hash| &file_hash == hash)
            .is_some();

        if is_already_analyzed {
            return Err(AppError::Anyhow(anyhow!(
                "Submitted file's hash is already saved in the KV store. Not runnning analysis."
            )));
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

        if let StatusCode::ACCEPTED = res.status() {
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
                        process_analysis_results(&file_hash, success_res, app_state.clone()).await
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
struct AllData {
    name: String,
    unit_price: f64,
    count: f64,
    merchant_name: String,
    paid_at: chrono::DateTime<chrono::Utc>,
}

async fn show_all(
    State(app_state): State<Arc<AppState>>,
) -> Result<axum::Json<Vec<AllData>>, AppError> {
    let pool = &app_state.pool;
    let data = sqlx::query_as!(AllData, "SELECT receipts.paid_at, receipts.merchant_name, prices.count, prices.unit_price, products.name FROM receipts JOIN prices ON receipts.id = prices.receipt_id JOIN products ON products.id = prices.product_id").fetch_all(pool).await?;
    Ok(axum::Json(data))
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[derive(Clone)]
struct AppState {
    client: Client,
    azure_form_recognizer_api_key: String,
    pool: PgPool,
    client_secret: String,
    persist: PersistInstance,
}

const UPLOAD_LIMIT_BYTES: usize = 1024 * 1024 * 10; // 10 MB

// Postgres maximum number of parameters in a statement
const BIND_LIMIT: usize = 65535;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_persist::Persist] persist: PersistInstance,
    #[shuttle_aws_rds::Postgres] pool: PgPool,
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> ShuttleAxum {
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(anyhow::Error::from)?;

    let Some(azure_form_recognizer_api_key) = secret_store.get("AZURE_FORM_RECOGNIZER_KEY") else {
        return Err(shuttle_runtime::Error::BuildPanic(
            "Could not find AZURE_FORM_RECOGNIZER_KEY in secrets".into(),
        ));
    };

    let Some(client_secret) = secret_store.get("CLIENT_SECRET") else {
        return Err(shuttle_runtime::Error::BuildPanic(
            "Could not find CLIENT_SECRET in secrets".into(),
        ));
    };

    for k in persist.list().unwrap() {
        tracing::info!("Stored result: {}", persist.load::<String>(&k).unwrap())
    }

    let client = Client::new();

    let app_state = AppState {
        client,
        azure_form_recognizer_api_key,
        pool,
        client_secret,
        persist,
    };

    let state = Arc::new(app_state);

    let router = Router::new()
        .route("/", get(hello_world))
        .route("/all", get(show_all))
        .route(
            "/upload",
            post(upload).layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES)),
        )
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);

    Ok(router.into())
    // tracing::info!("Response: {res:?}");
}

async fn auth<B>(
    State(app_state): State<Arc<AppState>>,
    axum::TypedHeader(axum::headers::Authorization(bearer)): axum::TypedHeader<
        axum::headers::Authorization<axum::headers::authorization::Bearer>,
    >,
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<axum::response::Response, StatusCode> {
    if app_state.client_secret != bearer.token() {
        return Err(StatusCode::FORBIDDEN);
    }
    let response = next.run(request).await;
    Ok(response)
}

async fn save_analysis_data(
    pool: &PgPool,
    analysis_result: AnalyzeResultOperation,
) -> Result<(), AppError> {
    let receipt_fields = analysis_result
        .analyzeResult
        .ok_or(anyhow!("Missing analyzeResult field"))?
        .documents
        .ok_or(anyhow!("Missing documents field"))?
        .first()
        .ok_or(anyhow!("Documents field is present but empty"))?
        .fields
        .clone();
    let (product_names, (counts, unit_prices)): (Vec<_>, (Vec<_>, Vec<_>)) = receipt_fields
        .items
        .value_array
        .iter()
        .filter_map(|item| {
            let Some(unit_price) = item.value_object.unit_price.as_ref().or(item.value_object.total_price.as_ref()).map(|obj| obj.value_number) else {
                // We throw away items where no price was detected
                return None;
            };
            let name = item.value_object.description.value_string.clone();
            let count = if let Some(q) = &item.value_object.quantity {
                q.value_number
            } else {
                1.0
            };
            Some((name, (count, unit_price)))
        })
        .into_iter()
        .take(BIND_LIMIT)
        .unzip();

    let datetime_str = receipt_fields.transaction_date.value_date
        + " "
        + &receipt_fields.transaction_time.value_time;
    let timestamp = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| anyhow!(format!("Invalid date string: {datetime_str}")))?;
    let chrono::LocalResult::Single(timestamp_tz) = Copenhagen.from_local_datetime(&timestamp)
    else {
        return Err(anyhow!("Error converting naive timestamp to Copenhagen time").into());
    };

    let merchant_name = &receipt_fields.merchant_name.value_string;

    let receipt_id = insert_receipt_if_not_exists(pool, merchant_name, timestamp_tz).await?;

    insert_products_if_not_exist(pool, &product_names)
        .await
        .map_err(AppError::from)?;

    insert_prices_for_products_and_receipt(pool, &counts, &unit_prices, &product_names, receipt_id)
        .await?;
    Ok(())
}

async fn insert_prices_for_products_and_receipt(
    pool: &PgPool,
    counts: &[f64],
    unit_prices: &[f64],
    product_names: &[String],
    receipt_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO prices(count, unit_price, receipt_id, product_id) SELECT UNNEST($1::float[]), UNNEST($2::float[]), $3, product_id FROM (SELECT id as product_id FROM products WHERE name IN (SELECT UNNEST($4::text[]))) tmp ON CONFLICT DO NOTHING"#,
        counts,
        unit_prices,
        receipt_id,
        product_names
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_receipt_if_not_exists(
    pool: &PgPool,
    merchant_name: &str,
    paid_at: chrono::DateTime<chrono_tz::Tz>,
) -> Result<i32, sqlx::Error> {
    let res = sqlx::query!(
        r#"INSERT INTO receipts(merchant_name, paid_at) VALUES ($1, $2) RETURNING *"#,
        merchant_name,
        paid_at
    )
    .fetch_one(pool)
    .await?
    .id;
    Ok(res)
    // row.try_get::<i32, &str>("id")
}

async fn insert_products_if_not_exist(
    pool: &PgPool,
    products: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"INSERT INTO products(name) SELECT DISTINCT new_name FROM (SELECT UNNEST($1::text[]):: text new_name) tmp WHERE new_name NOT IN (SELECT DISTINCT name FROM products)"#, products).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::manual;

    #[test]
    fn parse_receipt_analysis_results() {
        serde_json::from_str::<manual::AnalyzeResultOperation>(include_str!("../response1.json"))
            .unwrap();
    }

    #[test]
    fn parse_other_receipt_analysis_results() {
        serde_json::from_str::<manual::AnalyzeResultOperation>(include_str!("../response2.json"))
            .unwrap();
    }

    #[test]
    fn parse_another_receipt_analysis_results() {
        serde_json::from_str::<manual::AnalyzeResultOperation>(include_str!("../response3.json"))
            .unwrap();
    }
}
