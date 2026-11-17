use std::{path::Path, sync::Arc};

use anyhow::Context;
use axum::Router;
use raid_scheduler_backend::{
    api::{self, AppState, PngCache},
    auth::AuthSettings,
    bot,
    config::Config,
    db,
    services::clock::{Clock, SystemClock},
};
use tower_http::{
    compression::{
        CompressionLayer,
        predicate::{DefaultPredicate, NotForContentType, Predicate},
    },
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `.env` is optional; real deployments pass environment variables directly.
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Config::load().context("invalid configuration")?;
    log_startup(&config);

    let pool = db::connect(&config)
        .await
        .context("failed to initialize SQLite database")?;
    tracing::info!(
        database = %config.database_path.display(),
        "database ready (WAL, foreign keys, busy timeout verified)"
    );

    db::migrate(&pool)
        .await
        .context("failed to apply database migrations")?;
    tracing::info!("database migrations applied");

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let bot_task = start_telegram_bot(&config, &pool, clock.clone());

    let state = AppState {
        db: pool.clone(),
        auth: AuthSettings {
            bot_token: config.telegram_bot_token.clone(),
            max_age: config.init_data_max_age,
        },
        bot: bot_task.as_ref().map(|(bot, _)| bot.clone()),
        clock,
        png: Arc::new(PngCache::new()),
    };
    let app = with_static_frontend(api::router(state), &config.frontend_dist)
        .layer(compression_layer())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!(address = %listener.local_addr()?, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server error")?;

    // Stop long polling before the pool closes so the bot cannot race startup
    // state or use a closed database.
    if let Some((_, handle)) = bot_task {
        handle.abort();
    }

    // Closing the pool flushes WAL state cleanly on normal shutdown. It never
    // deletes the database file.
    pool.close().await;
    tracing::info!("shutdown complete");
    Ok(())
}

fn log_startup(config: &Config) {
    tracing::info!(
        bind_addr = %config.bind_addr,
        database = %config.database_path.display(),
        max_connections = config.db_max_connections,
        frontend_dist = %config.frontend_dist.display(),
        version = env!("CARGO_PKG_VERSION"),
        "starting raid scheduler backend"
    );
}

/// Runs the Telegram bot in the background when a token is configured.
///
/// Long polling is enough for the MVP and avoids exposing a webhook endpoint.
/// The token is never logged; `Config` redacts it in `Debug` output.
fn start_telegram_bot(
    config: &Config,
    pool: &sqlx::SqlitePool,
    clock: Arc<dyn Clock>,
) -> Option<(teloxide::Bot, tokio::task::JoinHandle<()>)> {
    let Some(token) = config.telegram_bot_token.clone() else {
        tracing::warn!("TELOXIDE_TOKEN is not set; Telegram bot is disabled");
        return None;
    };

    let bot = teloxide::Bot::new(token.expose());
    let polling_bot = bot.clone();
    let bot_pool = pool.clone();
    let bot_settings = bot::BotSettings {
        public_base_url: config.public_base_url.clone(),
        clock,
    };
    let handle = tokio::spawn(async move {
        bot::run(polling_bot, bot_pool, bot_settings).await;
    });
    tracing::info!("Telegram bot enabled (long polling)");
    Some((bot, handle))
}

/// Serves the Vite build when present. Missing builds are tolerated so the API
/// can run standalone during development (`npm run dev` proxies to it instead).
fn with_static_frontend(router: Router, dist: &Path) -> Router {
    let index = dist.join("index.html");
    if index.is_file() {
        tracing::info!(path = %dist.display(), "serving built frontend");
        router.fallback_service(ServeDir::new(dist).not_found_service(ServeFile::new(index)))
    } else {
        tracing::info!(
            path = %dist.display(),
            "frontend build not found; serving API only"
        );
        router
    }
}

/// Compresses API JSON and text assets. Images and already-compressed fonts are
/// skipped so CPU is not spent on incompressible payloads.
fn compression_layer() -> CompressionLayer<impl Predicate> {
    CompressionLayer::new()
        .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/")))
}

async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("shutdown signal received"),
        Err(error) => tracing::error!(error = %error, "failed to listen for shutdown signal"),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let structured = std::env::var("LOG_FORMAT")
        .map(|value| value.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let registry = tracing_subscriber::registry().with(filter);
    if structured {
        registry
            .with(tracing_subscriber::fmt::layer().json().with_target(true))
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .init();
    }
}
