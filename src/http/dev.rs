use std::{sync::Arc, time::Duration};

use axum::extract::State;

use crate::{error::AppError, http::analysis::RawResult, service::save_analysis_data};

use super::DbState;

// TODO: Remove this dev endpoint
pub async fn clear_db(State(DbState { pool }): State<DbState>) -> Result<&'static str, AppError> {
    let pool = &pool;
    let tx = pool.begin().await?;
    sqlx::query!("DELETE FROM prices").execute(pool).await?;
    sqlx::query!("DELETE FROM products").execute(pool).await?;
    sqlx::query!("DELETE FROM receipts").execute(pool).await?;
    tx.commit().await?;

    let msg = "All data has been deleted from DB";
    tracing::info!(msg);
    Ok(msg)
}

pub async fn repopulate_db_from_cache(
    State(DbState { pool }): State<DbState>,
) -> Result<&'static str, AppError> {
    let pool = &pool;
    let tx = pool.begin().await?;
    sqlx::query!("DELETE FROM prices").execute(pool).await?;
    sqlx::query!("DELETE FROM products").execute(pool).await?;
    sqlx::query!("DELETE FROM receipts").execute(pool).await?;
    tx.commit().await?;

    let raw_results = sqlx::query_as!(RawResult, "SELECT * FROM raw_results")
        .fetch_all(pool)
        .await?;
    let pool_ptr = Arc::new(pool.clone());
    tokio::spawn(async move {
        for RawResult {
            result_json,
            sha256_digest,
            ..
        } in raw_results
        {
            let pool = pool_ptr.clone();
            tokio::spawn(async move {
                let res = serde_json::from_str(&result_json).map_err(AppError::from);
                match res {
                    Ok(data) => {
                        if let Err(err) = save_analysis_data(&pool, data, &sha256_digest).await {
                            tracing::error!("{}", err.to_string());
                        } else {
                            tracing::info!(
                                "Successfully saved receipted data in DB for cached results of analyzing file {}",
                                sha256_digest
                            );
                        };
                    }
                    Err(err) => tracing::error!(
                        "Cached results for file {} encountered an error during processing: {}",
                        sha256_digest,
                        err.to_string()
                    ),
                };
            });
        }
    });
    let msg = "Successfully enqueued repopulation of DB data from cached analysis results. Results should be available shortly";
    tracing::info!(msg);
    Ok(msg)
}

pub async fn show_all_cached(
    State(DbState { pool }): State<DbState>,
) -> Result<axum::Json<Vec<RawResult>>, AppError> {
    let pool = &pool;
    let data = sqlx::query_as!(RawResult, "SELECT * FROM raw_results")
        .fetch_all(pool)
        .await?;
    Ok(axum::Json(data))
}
