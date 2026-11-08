mod account;
mod agent;
pub mod bootstrap;
mod catalog;
mod collection;
mod collector;
mod commands;
mod cycling;
mod dictionary;
mod effects;
mod figures;
mod gamelog;
mod gaps;
mod haul;
mod http;
mod layer;
mod logfile;
mod market;
mod mastery;
mod quarantine;
mod relics;
mod revision;
mod reward;
mod rivens;
mod roadmap;
mod schema;
mod screen;
mod settings;
#[cfg(feature = "dev")]
pub mod snapshot;
mod sound;
mod startup;
mod state_sink;
mod syndicates;
mod tasks;
mod update;
mod way;
mod window;
mod wiring;
mod worldstate;

use std::sync::Arc;

use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, Wry};

use crate::catalog::Layout;

/// URI scheme that serves item pictures from the game data to the window: `median-img://<id>`.
const PICTURES: &str = "median-img";

/// Builds and runs the Tauri application.
pub fn run() {
    init_tracing();
    logfile::log_panics();

    let builder = bootstrap::specta_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            window::bring_forward(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::Pending::default())
        .manage(startup::Failed::default())
        .invoke_handler(builder.invoke_handler())
        .register_uri_scheme_protocol(PICTURES, serve_picture)
        .setup(move |app| {
            builder.mount_events(app);
            let started =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wiring::init(app)));
            startup::settle(app, started);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Serves `median-img://<id>` with the WebP picture stored under that id in the game data;
/// 404 when there is none.
fn serve_picture(ctx: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    let layout = ctx.app_handle().state::<Arc<Layout>>();

    match catalog::picture(&layout, id) {
        Some(bytes) => Response::builder()
            .header(CONTENT_TYPE, "image/webp")
            .header(CACHE_CONTROL, "max-age=31536000")
            .body(bytes)
            .expect("picture response"),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new())
            .expect("picture response"),
    }
}

/// Installs a tracing subscriber filtered by `RUST_LOG` (default `info`) that writes the same
/// lines to stderr and, once `logfile::open` has set a path, to the log file.
fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::{EnvFilter, fmt, registry};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let to_file = fmt::layer().with_ansi(false).with_writer(logfile::ToFile);

    let _ = tracing::subscriber::set_global_default(
        registry().with(filter).with(fmt::layer()).with(to_file),
    );
}
