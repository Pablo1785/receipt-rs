use std::{collections::HashMap, str::FromStr, time::Duration};

use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Router,
};
use reqwest::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    Client, Request, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shuttle_axum::ShuttleAxum;
use shuttle_runtime::{self, Error};
use shuttle_secrets::SecretStore;

#[derive(Serialize, Deserialize)]
struct AnalyzeRequestBody {
    base64Source: String,
}

const ENDPOINT: &str = "https://receipt-model.cognitiveservices.azure.com/";
const MODEL_ID: &str = "prebuilt-receipt";

async fn analyze_file(file_bytes: Vec<u8>, api_key: &str, client: Client) -> Response {
    let url = format!(
        "{ENDPOINT}formrecognizer/documentModels/{MODEL_ID}:analyze?api-version=2023-07-31"
    );
    let req = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .body(json!({ "base64Source": file_bytes }).to_string())
        .build()
        .unwrap();
    client.execute(req).await.unwrap()
}

async fn get_analysis_results(analysis_id: &str, api_key: &str, client: Client) -> Response {
    let url = format!("{ENDPOINT}/formrecognizer/documentModels/{MODEL_ID}/analyzeResults/{analysis_id}?api-version=2023-07-31");
    let req = client
        .get(url)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, "0")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .build()
        .unwrap();
    client.execute(req).await.unwrap()
}

async fn analyze(State(api_key): State<&str>, mut multipart: Multipart) -> &'static str {
    let file_bytes = multipart.next_field();

    let client = Client::new();

    // analyze_file(file_bytes, api_key, client).await;

    // tokio::spawn(async {
    //         tokio::time::sleep(Duration::from_secs(30)).await;
    //         get_analysis_results(analysis_id, api_key, client).await;
    //     }
    // );

    "Hello"
}

async fn upload(mut multipart: Multipart) -> () {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!("Length of `{}` is {} bytes", name, data.len());
    }
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[shuttle_runtime::main]
async fn main(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> ShuttleAxum {
    let Some(key) = secret_store.get("AZURE_FORM_RECOGNIZER_KEY") else {
        return Err(Error::BuildPanic("Could not find AZURE_FORM_RECOGNIZER_KEY in secrets".into()));
    };
    // let post_req = r#"curl -v -i POST "{endpoint}/formrecognizer/documentModels/{modelID}:analyze?api-version=2023-07-31" -H "Content-Type: application/json" -H "Ocp-Apim-Subscription-Key: {key}" --data-ascii "{'urlSource': '{your-document-url}'}""#;

    // let file_bytes = include_str!("../b64file.txt");
    // let url = format!("{ENDPOINT}/formrecognizer/documentModels/{MODEL_ID}:analyze?api-version=2023-07-31");
    // let client = Client::new();
    // let req = client.post(url).header(CONTENT_TYPE, "application/json").header("Ocp-Apim-Subscription-Key", api_key).body(json!({ "base64Source": file_bytes }).to_string()).build().unwrap();

    // let res = analyze_file("../b64file.txt").await;

    // let res = get_analysis_results("518579fc-847d-4744-9d3b-f7b8114420a3").await;
    // let response_bytes = tokio::fs::read("response.txt").await.unwrap();
    // let data = serde_json::Value::from_str(&String::from_utf8(response_bytes).unwrap()).unwrap();
    // println!("{data}");
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/upload", post(upload));

    Ok(router.into())
    // println!("Response: {res:?}");
}
