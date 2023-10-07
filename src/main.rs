use chrono::{FixedOffset, TimeZone};
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

mod generated;
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

async fn analyze(State(_api_key): State<&str>, mut multipart: Multipart) -> &'static str {
    let _file_bytes = multipart.next_field();

    let _client = Client::new();

    // analyze_file(file_bytes, api_key, client).await;

    // tokio::spawn(async {
    //         tokio::time::sleep(Duration::from_secs(30)).await;
    //         get_analysis_results(analysis_id, api_key, client).await;
    //     }
    // );

    "Hello"
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

async fn upload(
    State(app_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(), AppError> {
    println!("Upload endpoint hit!");
    if let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();
        let filename = field.file_name().unwrap_or_default().to_string();
        let data = field.bytes().await?;

        println!(
            "Length of `{}` (file: {}) is {} bytes",
            name,
            filename,
            data.len()
        );

        let base64_file = BASE64_STANDARD.encode(data);

        let res = analyze_file(
            &base64_file,
            &app_state.azure_form_recognizer_api_key,
            &app_state.client,
        )
        .await?;

        println!("Analysis Response: {res:?}");
        if let StatusCode::ACCEPTED = res.status() {
            let result_url = res
                .headers()
                .get("Operation-Location")
                .ok_or(anyhow!(
                    "Missing Operation-Location in response header. This should never happen"
                ))?
                .to_str()?
                .to_string();
            println!(
                "Successfully queued image analysis. Result will be available at: {result_url}"
            );

            tokio::spawn(async move {
                println!("Waiting before asking for results...");
                tokio::time::sleep(Duration::from_secs(30)).await;
                let res = get_analysis_results(
                    &result_url,
                    &app_state.azure_form_recognizer_api_key,
                    &app_state.client,
                )
                .await;
                match res {
                    Ok(success_res) => {
                        let text = success_res.text().await?;
                        let data: manual::AnalyzeResultOperation = serde_json::from_str(&text)?;
                        save_analysis_data(&app_state.pool, data).await?;
                        println!("Successfully saved receipt data");
                        Ok::<(), AppError>(())
                    }
                    Err(err) => {
                        println!("{}", err.to_string());
                        Ok(())
                    }
                }
            });
        }
    }
    Ok(())
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
}

const UPLOAD_LIMIT_BYTES: usize = 1024 * 1024 * 10; // 10 MB

// Postgres maximum number of parameters in a statement
const BIND_LIMIT: usize = 65535;

#[shuttle_runtime::main]
async fn main(
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
    // let post_req = r#"curl -v -i POST "{endpoint}/formrecognizer/documentModels/{modelID}:analyze?api-version=2023-07-31" -H "Content-Type: application/json" -H "Ocp-Apim-Subscription-Key: {key}" --data-ascii "{'urlSource': '{your-document-url}'}""#;

    // let file_bytes = include_str!("../b64file.txt");
    // let url = format!("{ENDPOINT}/formrecognizer/documentModels/{MODEL_ID}:analyze?api-version=2023-07-31");
    // let client = Client::new();
    // let req = client.post(url).header(CONTENT_TYPE, "application/json").header("Ocp-Apim-Subscription-Key", api_key).body(json!({ "base64Source": file_bytes }).to_string()).build().unwrap();

    // let res = analyze_file("../b64file.txt").await;

    // let res = get_analysis_results("https://receipt-model.cognitiveservices.azure.com/formrecognizer/documentModels/prebuilt-receipt/analyzeResults/e4f52411-cc8e-491d-bbfe-78d4d690749f?api-version=2023-07-31", &azure_form_recognizer_api_key, &Client::new()).await.unwrap().text().await.unwrap();
    // let response_bytes = tokio::fs::read("response.json").await.unwrap();
    // let data: Root = serde_json::from_str(&String::from_utf8(response_bytes).unwrap()).unwrap();
    // let data: Value = serde_json::from_str(&String::from_utf8(response_bytes).unwrap()).unwrap();
    // let item_prices = get_item_prices(data);
    // let item_prices = data.analyze_result.documents.first().unwrap().fields.items.value_array.iter().map(|obj| (obj.value_object.description.content.clone(), obj.value_object.total_price.value_number)).collect::<Vec<(String, f64)>>();
    // println!("{item_prices:?}");

    // let json = json!({"MerchantName": "abcdefg"});

    // println!("Products OK: {products:?}");
    // let analysis_result = serde_json::from_str::<manual::AnalyzeResultOperation>(include_str!("../response.json"))
    // .unwrap();
    // save_analysis_data(&pool, analysis_result).await.unwrap();

    // let data = sqlx::query!("SELECT receipts.paid_at, receipts.merchant_name, prices.*, products.* FROM receipts JOIN prices ON receipts.id = prices.receipt_id JOIN products ON products.id = prices.product_id").fetch_all(&pool).await.unwrap();
    // println!("ROWS: {data:?}");
    let client = Client::new();

    let app_state = AppState {
        client,
        azure_form_recognizer_api_key,
        pool,
        client_secret,
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
    // println!("Response: {res:?}");
}

async fn auth<B>(
    State(app_state): State<Arc<AppState>>,
    axum::TypedHeader(axum::headers::Authorization(bearer)): axum::TypedHeader<
        axum::headers::Authorization<axum::headers::authorization::Bearer>,
    >,
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<axum::response::Response, StatusCode>{
    if app_state.client_secret != bearer.token() {
        return Err(StatusCode::FORBIDDEN);
    }

    let response = next.run(request).await;

    // do something with `response`...

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
        .map(|item| {
            let name = item.value_object.description.value_string.clone();
            let count = item.value_object.quantity.unwrap_or(1.0);
            let unit_price = item
                .value_object
                .unit_price
                .unwrap_or(item.value_object.total_price.value_number);
            (name, (count, unit_price))
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

fn get_item_prices(val: Value) -> anyhow::Result<Vec<(String, i64, f64)>> {
    // let item_prices = data.analyze_result.documents.first().unwrap().fields.items.value_array.iter().map(|obj| (obj.value_object.description.content.clone(), obj.value_object.total_price.value_number)).collect::<Vec<(String, f64)>>();
    val.get("analyzeResult")
        .ok_or(anyhow!("Missing analyzeResult"))?
        .get("documents")
        .ok_or(anyhow!("Missing documents"))?
        .as_array()
        .ok_or(anyhow!("documents not an array"))?
        .first()
        .ok_or(anyhow!("Empty documents"))?
        .get("fields")
        .ok_or(anyhow!("Missing fields"))?
        .get("Items")
        .ok_or(anyhow!("Missing Items"))?
        .get("valueArray")
        .ok_or(anyhow!("Missing valueArray"))?
        .as_array()
        .ok_or(anyhow!("valueArray not an array"))?
        .iter()
        .map(get_item_price_from_value_obj)
        .collect()
}

fn get_item_price_from_value_obj(obj: &Value) -> anyhow::Result<(String, i64, f64)> {
    let value_obj = obj
        .get("valueObject")
        .ok_or(anyhow!("Missing valueObject"))?;
    let name = value_obj
        .get("Description")
        .ok_or(anyhow!("Missing Description"))?
        .get("content")
        .ok_or(anyhow!("Missing content"))?
        .as_str()
        .ok_or(anyhow!("content not a string"))?;
    if let Some(quantity) = value_obj.get("Quantity").and_then(|v| v.as_i64()) {
        let unit_price = value_obj
            .get("UnitPrice")
            .ok_or(anyhow!("Missing UnitPrice"))?
            .get("valueNumber")
            .ok_or(anyhow!("Missing valueNumber"))?
            .as_f64()
            .ok_or(anyhow!("UnitPrice not a float64"))?;
        Ok((name.to_string(), quantity, unit_price))
    } else {
        let total_price = value_obj
            .get("TotalPrice")
            .ok_or(anyhow!("Missing TotalPrice"))?
            .get("valueNumber")
            .ok_or(anyhow!("Missing valueNumber"))?
            .as_f64()
            .ok_or(anyhow!("TotalPrice not a float64"))?;
        Ok((name.to_string(), 1, total_price))
    }
}

#[cfg(test)]
mod tests {
    use crate::manual;

    #[test]
    fn parse_receipt_analysis_results() {
        serde_json::from_str::<manual::AnalyzeResultOperation>(include_str!("../response.json"))
            .unwrap();
    }
}
