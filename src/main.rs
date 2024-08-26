use chrono::TimeZone;
use http::{app, serve, AppDeps};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::trace::{self, DefaultMakeSpan, TraceLayer};
use tracing::{info_span, instrument::WithSubscriber, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use axum::{extract::multipart::MultipartError, http::StatusCode, response::IntoResponse};

use chrono_tz::Europe::Copenhagen;
use itertools::Itertools;
use manual::AnalyzeResultOperation;
use reqwest::{
    header::{ToStrError, CONTENT_LENGTH, CONTENT_TYPE},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::migrate::MigrateError;
use thiserror::Error;

mod http;
mod manual;
mod service;

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
    Csv(#[from] csv::Error),
    #[error(transparent)]
    StringFromUtf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    CsvIntoInner(#[from] csv::IntoInnerError<csv::Writer<Vec<u8>>>),
    #[error(transparent)]
    EnvVar(#[from] std::env::VarError),
    #[error(transparent)]
    Migrate(#[from] MigrateError),
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

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let deps = AppDeps::try_from_env().await?;

    MIGRATOR.run(&deps.pool).await?;

    serve(deps).await
}
