use std::{str::FromStr, sync::Arc};

use app_config::ServerConfig;
use axum::{routing::get, Extension, Router};

use tracing::{debug, info};

use anyhow::Result;
use argparse::{ArgumentParser, Store};
use dotenv::dotenv;
use tracing_subscriber::{filter, prelude::*};

mod app_config;
mod download;
mod errors;
mod routes;
mod storage;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok(); // This line loads the environment variables from the ".env" file.

    let mut config_path = "".to_string();
    {
        // limit scope of argparse borrows
        let mut ap = ArgumentParser::new();
        ap.set_description("A Minecraft metadata api server for Mojang and Modloader metadata.");
        ap.refer(&mut config_path).add_option(
            &["-c", "--config"],
            Store,
            "Path to a json config file.",
        );
        ap.parse_args_or_exit();
    }

    let config = Arc::new(ServerConfig::from_config(&config_path)?);

    let file_appender =
        tracing_appender::rolling::hourly(&config.debug_log.path, &config.debug_log.prefix);
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);
    let stdout_log = tracing_subscriber::fmt::layer().compact();

    let debug_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking_file)
        .with_filter(filter::LevelFilter::from_level(
            tracing::Level::from_str(&config.debug_log.level).unwrap_or(tracing::Level::DEBUG),
        ));

    if config.debug_log.enable {
        tracing_subscriber::registry()
            .with(stdout_log.with_filter(filter::EnvFilter::from_default_env()))
            .with(debug_log)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(stdout_log.with_filter(filter::EnvFilter::from_default_env()))
            .init();
    }

    debug!("Config: {:#?}", config);

    config
        .storage_format
        .initialize_metadata(&config.metadata)
        .await?;

    let raw_mojang_routes = Router::new()
        .route("/", get(routes::mojang::raw_mojang_manifest))
        .route("/:version", get(routes::mojang::raw_mojang_version));
    let raw_forge_routes = Router::new()
        .route("/", get(routes::forge::raw_forge_maven_meta))
        .route("/promotions", get(routes::forge::raw_forge_promotions))
        .route("/:version", get(routes::forge::raw_forge_version))
        .route("/:version/meta", get(routes::forge::raw_forge_version_meta))
        .route(
            "/:version/installer",
            get(routes::forge::raw_forge_version_installer),
        );

    let raw_routes = Router::new()
        .nest("/mojang", raw_mojang_routes)
        .nest("/forge", raw_forge_routes);

    let http = Router::new()
        .nest("/raw", raw_routes)
        .layer(Extension(config.clone()));

    let addr = config.bind_address.parse()?;
    info!("Starting server on {}", addr);
    axum::Server::bind(&addr)
        .serve(http.into_make_service())
        .await?;

    Ok(())
}
