#![allow(clippy::upper_case_acronyms)]
// Many module types are intentionally defined but used only in specific
// runtime paths (e.g., enriched vs. mock data, optional features).
#![allow(dead_code)]

mod bootstrap;
mod cilium;
mod ebpf;
mod endpoints;
mod hubble;
mod integration;
mod kubernetes;
mod modules;
mod policies;
mod tui;

use anyhow::Result;
use clap::Parser;
use tracing::Level;

use bootstrap::BootstrapManager;
use tui::TuiApp;

#[derive(Parser, Debug)]
#[command(name = "cilium-tui")]
#[command(about = "Zero-touch Cilium observability TUI with automatic bootstrapping", long_about = None)]
struct Args {
    /// Skip bootstrap and go straight to TUI
    #[arg(long)]
    skip_bootstrap: bool,

    /// Hubble port (default: 4245)
    #[arg(long, default_value = "4245")]
    hubble_port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Automatically install Cilium if not present (no prompts)
    #[arg(long)]
    auto_install: bool,

    /// Automatically upgrade Cilium to latest version
    #[arg(long)]
    auto_upgrade: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    let (context, hubble_port, k8s_client) = if args.skip_bootstrap {
        // Skip bootstrap, use provided port
        let k8s_client = kubernetes::K8sClient::new().await?;
        ("unknown".to_string(), args.hubble_port, k8s_client)
    } else {
        // Run full bootstrap
        let bootstrap = BootstrapManager::new().await?;
        let k8s_client_clone = bootstrap.get_k8s_client();
        let result = bootstrap
            .run_bootstrap_with_options(args.auto_install, args.auto_upgrade)
            .await?;
        (result.context, result.hubble_port, k8s_client_clone)
    };

    // Launch TUI
    let mut app = TuiApp::new(context, hubble_port, k8s_client).await?;
    app.run().await?;

    Ok(())
}
