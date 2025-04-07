use std::{any::Any, net::SocketAddr, sync::Arc};

use analysis::{download, show_all, upload};
use anyhow::Context as _;
use axum::{
    error_handling::HandleErrorLayer,
    extract::{DefaultBodyLimit, FromRef, MatchedPath},
    http::Request,
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use dev::{clear_cache, clear_db, repopulate_db_from_cache, show_all_cached};
use reqwest::{Client, ClientBuilder};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceBuilder;
use tower_http::{
    classify::ServerErrorsFailureClass,
    trace::{DefaultOnFailure, OnFailure as _, TraceLayer},
};
use tracing::{debug_span, error_span, info_span, Span};

use crate::{error::AppError, service::ocr::OcrDeps};

mod analysis;
mod auth;
mod dev;

pub use analysis::{AllData, WAIT_BEFORE_ASKING_FOR_RESULTS};

const UPLOAD_LIMIT_BYTES: usize = 1024 * 1024 * 10; // 10 MB

pub struct AppDeps {
    pub pool: PgPool,
    pub ocr: OcrDeps,
    pub client_secret: String,
    pub client: Client,
}

impl AppDeps {
    pub async fn try_from_env() -> Result<Self, AppError> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL env var missing")?;
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(&database_url)
            .await?;

        let client_secret =
            std::env::var("CLIENT_SECRET").context("CLIENT_SECRET env var missing")?;

        let deps = AppDeps {
            pool,
            ocr: OcrDeps::try_from_env()?,
            client_secret,
            client: ClientBuilder::new().use_rustls_tls().build()?,
        };
        Ok(deps)
    }

    /// This function is used for testing purposes only
    pub async fn try_from_env_and_pool(pool: PgPool) -> Result<Self, AppError> {
        let client_secret =
            std::env::var("CLIENT_SECRET").context("CLIENT_SECRET env var missing")?;

        let deps = AppDeps {
            pool,
            ocr: OcrDeps::try_from_env()?,
            client_secret,
            client: Client::new(),
        };
        Ok(deps)
    }
}

pub struct DbState {
    pub pool: PgPool,
}

impl FromRef<Arc<AppDeps>> for DbState {
    fn from_ref(input: &Arc<AppDeps>) -> Self {
        Self {
            pool: input.pool.clone(),
        }
    }
}

pub fn app(deps: AppDeps) -> Router {
    let deps = Arc::new(deps);
    let router = Router::new()
        .route("/dev/db/all", delete(clear_db).with_state(deps.clone()))
        .route(
            "/dev/db/all",
            put(repopulate_db_from_cache).with_state(deps.clone()),
        )
        .route(
            "/dev/cache/all",
            get(show_all_cached).with_state(deps.clone()),
        )
        .route(
            "/dev/cache/all",
            delete(clear_cache).with_state(deps.clone()),
        )
        .route("/all", get(show_all).with_state(deps.clone()))
        .route(
            "/upload",
            post(upload)
                .with_state(deps.clone())
                .layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES)),
        )
        .route("/download", get(download).with_state(deps.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    debug_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        status_code = tracing::field::Empty,
                        error_msg = tracing::field::Empty,
                        duration_ms = tracing::field::Empty,
                    )
                })
                .on_failure(()),
        )
        .layer(axum::middleware::from_fn_with_state(
            deps.clone(),
            auth::auth,
        ));
    router
}

pub async fn serve(deps: AppDeps) -> Result<(), AppError> {
    let router = app(deps);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Binding TCP listener to addr")?;
    tracing::info!("Server listening on {}", addr);
    axum::serve(listener, router.into_make_service())
        .await
        .context("Server must always start listening")?;
    Ok(())
}

pub async fn handle_app_error(err: AppError) -> (axum::http::StatusCode, String) {
    match err {
        error @ _ => {
            tracing::error!("HTTP client error: {}", error);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("HTTP client error: {}", error),
            )
        }
    }
}
