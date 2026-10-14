#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod application;
mod config;
mod domain;
mod i18n;
mod logging;
mod output;
mod platform;
mod ports;
mod ui;

fn main() -> anyhow::Result<()> {
    let log_path = logging::initialize(&config::Config::path().with_file_name("logs"))?;
    eprintln!("ShiPing log: {}", log_path.display());
    log::info!(
        "Starting ShiPing {} on {}-{}; GPUI Kit 0.7.3; debug={}; log={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(debug_assertions),
        log_path.display()
    );
    let result = ui::run();
    match &result {
        Ok(()) => log::info!("Application exited normally"),
        Err(error) => log::error!("Application failed: {error:#}"),
    }
    log::logger().flush();
    result
}
