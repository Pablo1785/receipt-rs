use std::{path::Path, time::Duration};

use axum::http::StatusCode;
use common::RequestBuilderExt as _;
use receipt_rs::http::{AllData, WAIT_BEFORE_ASKING_FOR_RESULTS};
use reqwest::{
    multipart::{Form, Part},
    Url,
};
use tokio::{fs::File, io::AsyncReadExt};

mod common;

#[cfg(feature = "test_db")]
#[sqlx::test]
async fn test_download_csv(db: PgPool) {
    let deps = AppDeps::try_from_env_and_pool(db).await.unwrap();
    let client_secret = deps.client_secret.clone();
    let mut app = app(deps);

    // Happy path!
    let resp1 = app
        .borrow_mut()
        // We handle JSON objects directly to sanity check the serialization and deserialization
        .oneshot(
            Request::get("/download")
                .header(
                    axum::http::header::AUTHORIZATION,
                    format!("Bearer {}", client_secret),
                )
                .empty_body(),
        )
        .await
        .unwrap();

    assert_eq!(resp1.status(), StatusCode::NO_CONTENT);
}

async fn test_upload(client_secret: &str, path: &Path) {
    let client = reqwest::Client::new();
    let url = Url::parse("http://localhost:8080/upload").unwrap();
    let request = reqwest::Request::new(reqwest::Method::POST, url);

    let expected_bytes = 1024 * 1024 * 3;
    let mut buf = Vec::with_capacity(expected_bytes);
    File::open(path)
        .await
        .expect("File open failed")
        .read_to_end(&mut buf)
        .await
        .expect("File data read failed");
    let resp = reqwest::RequestBuilder::from_parts(client, request)
        .bearer_auth(&client_secret)
        .multipart(Form::new().part("receipt_photo", Part::bytes(buf)))
        .send()
        .await
        .unwrap();

    tracing::info!("{:?}", resp);

    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    tokio::time::sleep(WAIT_BEFORE_ASKING_FOR_RESULTS + Duration::from_secs(10)).await;
    let client = reqwest::Client::new();
    let url = Url::parse("http://localhost:8080/all").unwrap();
    let request = reqwest::Request::new(reqwest::Method::GET, url);
    let resp = reqwest::RequestBuilder::from_parts(client, request)
        .bearer_auth(client_secret)
        .send()
        .await
        .unwrap();

    tracing::info!("{:?}", resp);

    assert_eq!(resp.status(), StatusCode::OK);

    let rows = resp.json::<Vec<AllData>>().await.unwrap();
    tracing::info!("{:?}", rows);
    assert!(!rows.is_empty());
}

#[cfg(feature = "test_db")]
#[sqlx::test]
async fn test_upload_images(db: PgPool) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let deps = AppDeps::try_from_env_and_pool(db).await.unwrap();
    let client_secret = deps.client_secret.clone();
    tokio::spawn(serve(deps));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let img_dir = Path::new("tests").join("fixtures").join("receipts");
    assert!(img_dir.is_dir());

    for entry in img_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            test_upload(&client_secret, &path).await;
        }
    }
}
