use receipt_rs::{error::AppError, http::serve, http::AppDeps};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
