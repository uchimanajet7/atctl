mod app;
mod at;
mod cli;
mod config;
mod log;
mod presets;
mod sequences;
mod transport;
mod tui;
mod usb;

pub use app::errors::{AtctlError, Result};

pub fn run() -> Result<()> {
    cli::run()
}

pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .try_init();
}
