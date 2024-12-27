use std::borrow::BorrowMut;

use axum::http::{Request, StatusCode};
use common::RequestBuilderExt as _;
use receipt_rs::http::{app, AppDeps};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

mod common;

#[sqlx::test]
async fn test_download_csv(db: PgPool) {
    let deps = AppDeps::try_from_env_and_pool(db).await.unwrap();
    let client_secret = deps.client_secret.clone();
    let mut app = app(deps);

    // Happy path!
    let resp1 = app
        .borrow_mut()
        // We handle JSON objects directly to sanity check the serialization and deserialization
        .oneshot(Request::get("/download").header(axum::http::header::AUTHORIZATION, format!("Bearer {}", client_secret)).empty_body())
        .await
        .unwrap();

    assert_eq!(resp1.status(), StatusCode::NO_CONTENT);
}
