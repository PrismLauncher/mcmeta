use std::{str::FromStr, sync::Arc};

use app_config::MetaConfig;
use axum::{routing::get, Router};

use routes::ServerState;
use storage::StorageImpl;
use tracing::{info, warn};

use indicatif::ProgressState;
use indicatif::ProgressStyle;
use tracing::instrument;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use tokio::signal::ctrl_c;
#[cfg(target_family = "unix")]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(target_family = "windows")]
use tokio::signal::windows::ctrl_close;

use dotenv::dotenv;
use eyre::Result;
use tracing_subscriber::{filter, prelude::*};

mod app_config;
mod errors;
mod routes;
mod storage;
mod upstream;
mod utils;

#[macro_use]
extern crate lazy_static;

use clap::Parser;
use upstream::UpstreamMetadataUpdater;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,
    #[arg(long)]
    use_dotenv: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, strum::EnumIter)]
pub enum UpstreamSources {
    Mojang,
    Forge,
    All,
}
pub fn all_upstream_sources() -> Vec<UpstreamSources> {
    use strum::IntoEnumIterator;
    UpstreamSources::iter()
        .filter(|i| !matches!(i, UpstreamSources::All))
        .collect()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, strum::EnumIter)]
pub enum GeneratedSources {
    All,
}
pub fn all_generated_sources() -> Vec<GeneratedSources> {
    use strum::IntoEnumIterator;
    GeneratedSources::iter()
        .filter(|i| !matches!(i, GeneratedSources::All))
        .collect()
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Fetch {
        #[arg(value_enum, short, long, default_values_t = all_upstream_sources())]
        sources: Vec<UpstreamSources>,
    },
    Generate {
        #[arg(value_enum, short, long, default_values_t = all_generated_sources())]
        sources: Vec<GeneratedSources>,
    },
    FetchAndGenerate {
        #[arg(value_enum, short, long, default_values_t = all_upstream_sources())]
        fetch: Vec<UpstreamSources>,
        #[arg(value_enum, short, long, default_values_t = all_generated_sources())]
        generate: Vec<GeneratedSources>,
    },
    Serve {
        bind_address: Option<String>,
    },
}
fn elapsed_subsec(state: &ProgressState, writer: &mut dyn std::fmt::Write) {
    let seconds = state.elapsed().as_secs();
    let sub_seconds = (state.elapsed().as_millis() % 1000) / 100;
    let _ = writer.write_str(&format!("{}.{}s", seconds, sub_seconds));
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut config_path = String::from("./mcmeta.toml");

    let args = CliArgs::parse();

    if args.use_dotenv {
        dotenv().ok();
    }

    if let Some(path) = args.config {
        config_path = path;
    }

    let config: Arc<MetaConfig> = Arc::new(MetaConfig::from_config(&config_path)?);

    let file_appender =
        tracing_appender::rolling::hourly(&config.debug_log.path, &config.debug_log.prefix);
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    let tracing_filter_level =
        tracing::Level::from_str(&config.debug_log.level).unwrap_or(tracing::Level::INFO);

    let indicatif_layer = IndicatifLayer::new()
        .with_progress_style(
            ProgressStyle::with_template(
                "{span_child_prefix}{span_fields} -- {span_name} {wide_msg} {elapsed_subsec}",
            )
            .unwrap()
            .with_key("elapsed_subsec", elapsed_subsec),
        )
        .with_span_child_prefix_symbol("↳ ")
        .with_span_child_prefix_indent(" ");

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(indicatif_layer.get_stdout_writer())
        .with_filter(filter::LevelFilter::from_level(tracing_filter_level));

    let debug_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking_file)
        .with_filter(filter::LevelFilter::from_level(tracing_filter_level));

    if config.debug_log.enable {
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(indicatif_layer)
            .with(debug_log)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(indicatif_layer)
            .init();
    }

    info!("Config: {:#?}", config);

    let upstream_storage = Arc::new(config.upstream.storage_format.build().await?);
    let generated_storage = Arc::new(config.generated.storage_format.build().await?);

    let command = args.command.unwrap_or_else(|| Command::FetchAndGenerate {
        fetch: all_upstream_sources(),
        generate: all_generated_sources(),
    });

    #[cfg(target_family = "unix")]
    let mut sigterm = signal(SignalKind::terminate())?;
    #[cfg(target_family = "windows")]
    let mut sigterm = ctrl_close()?;

    tokio::select! {
        result = run(command, config, upstream_storage, generated_storage) => result,
        _ = sigterm.recv() => {
            handle_shutdown("Received SIGTERM");
            std::process::exit(0);
        }
        _ = ctrl_c() => {
            handle_shutdown("Interrupted");
            std::process::exit(130);
        }
    }
}

fn handle_shutdown(reason: &str) {
    warn!("{reason}! Shutting down ...");
    println!("Everything is shutdown. Goodbye!");
}

async fn run(
    command: Command,
    config: Arc<MetaConfig>,
    upstream_storage: Arc<StorageImpl>,
    generated_storage: Arc<StorageImpl>,
) -> Result<()> {
    match command {
        Command::Fetch { sources } => {
            let sources = if sources.contains(&UpstreamSources::All) {
                all_upstream_sources()
            } else {
                sources
            };
            fetch(config, upstream_storage, &sources).await
        }
        Command::Generate { sources } => {
            let sources = if sources.contains(&GeneratedSources::All) {
                all_generated_sources()
            } else {
                sources
            };
            generate(config, generated_storage, &sources).await
        }
        Command::FetchAndGenerate {
            fetch: to_fetch,
            generate: to_generate,
        } => {
            let to_fetch = if to_fetch.contains(&UpstreamSources::All) {
                all_upstream_sources()
            } else {
                to_fetch
            };
            let to_generate = if to_generate.contains(&GeneratedSources::All) {
                all_generated_sources()
            } else {
                to_generate
            };
            fetch(config.clone(), upstream_storage.clone(), &to_fetch).await?;
            generate(config, generated_storage, &to_generate).await
        }
        Command::Serve { bind_address } => {
            serve(bind_address, config, upstream_storage, generated_storage).await
        }
    }
}

#[instrument(skip_all)]
async fn fetch(
    config: Arc<MetaConfig>,
    storage: Arc<StorageImpl>,
    sources: &[UpstreamSources],
) -> Result<()> {
    UpstreamMetadataUpdater::new(config.upstream.clone(), storage)
        .update(sources)
        .await
}

#[allow(dead_code)]
#[instrument(skip_all)]
async fn generate(
    config: Arc<MetaConfig>,
    storage: Arc<StorageImpl>,
    sources: &[GeneratedSources],
) -> Result<()> {
    Ok(())
}

#[instrument(skip_all)]
async fn serve(
    bind_address: Option<String>,
    config: Arc<MetaConfig>,
    upstream_storage: Arc<StorageImpl>,
    generated_storage: Arc<StorageImpl>,
) -> eyre::Result<()> {
    // config
    //     .storage_format
    //     .update_upstream_metadata(&config.metadata)
    //     .await?;

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

    let addr = bind_address.unwrap_or_else(|| config.bind_address.clone());

    let http = Router::new()
        .nest("/raw", raw_routes)
        .with_state(Arc::new(ServerState {
            config,
            upstream_storage,
            generated_storage,
        }));

    info!("Starting server on {}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, http).await?;

    Ok(())
}
