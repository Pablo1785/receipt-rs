use receipt_rs::{error::AppError, http::serve, http::AppDeps};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

async fn run() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let deps = AppDeps::try_from_env().await?;

    MIGRATOR.run(&deps.pool).await?;

    serve(deps).await
}

fn main() {
    let sentry_dsn = std::env::var("SENTRY_DSN").expect("SENTRY_DSN env var missing");
    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run())
        .unwrap();
}
