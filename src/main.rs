#![allow(clippy::upper_case_acronyms)]
#![allow(dead_code)]

mod bootstrap;
mod cilium;
mod cli;
mod ebpf;
mod endpoints;
mod hubble;
mod integration;
mod kubernetes;
mod modules;
mod policies;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::Level;

use bootstrap::BootstrapManager;
use cli::InstallOpts;
use tui::TuiApp;

#[derive(Parser, Debug)]
#[command(name = "paqtra")]
#[command(about = "Paqtra — trace every flow. Network observability for Kubernetes")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(propagate_version = true)]
#[command(disable_help_subcommand = false)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install Paqtra in a Kubernetes cluster using Helm
    Install {
        /// Target namespace
        #[arg(long, default_value = "paqtra")]
        namespace: String,
        /// Path to Helm chart directory
        #[arg(long)]
        chart_directory: Option<PathBuf>,
        /// Helm chart version
        #[arg(long)]
        version: Option<String>,
        /// Wait for resources to become ready
        #[arg(long, default_value_t = true)]
        wait: bool,
        /// Helm --set key=value (repeatable)
        #[arg(long = "set")]
        set: Vec<String>,
    },
    /// Upgrade a Paqtra installation
    Upgrade {
        #[arg(long, default_value = "paqtra")]
        namespace: String,
        #[arg(long)]
        chart_directory: Option<PathBuf>,
        #[arg(long, default_value_t = true)]
        wait: bool,
        #[arg(long = "set")]
        set: Vec<String>,
    },
    /// Uninstall Paqtra using Helm
    Uninstall {
        #[arg(long, default_value = "paqtra")]
        namespace: String,
    },
    /// Display status (Cilium-style banner)
    Status {
        #[arg(long, default_value = "paqtra")]
        namespace: String,
        /// Wait until components are ready
        #[arg(long)]
        wait: bool,
        /// Output format: summary | json
        #[arg(short, long, default_value = "summary")]
        output: String,
    },
    /// Show cluster / install info
    Info {
        #[arg(long, default_value = "paqtra")]
        namespace: String,
    },
    /// Display feature discovery catalog (observe tiers)
    Features {
        /// Output format: summary | json
        #[arg(short, long, default_value = "summary")]
        output: String,
    },
    /// Read-only BPF inventory and drift (never attaches)
    Ebpf {
        #[command(subcommand)]
        command: EbpfCommands,
    },
    /// Display version information
    Version,
    /// Launch the interactive TUI
    Tui {
        #[arg(long)]
        skip_bootstrap: bool,
        #[arg(long, default_value = "4245")]
        hubble_port: u16,
        #[arg(long)]
        auto_install: bool,
        #[arg(long)]
        auto_upgrade: bool,
    },
    /// Run the node agent (DaemonSet entrypoint)
    Agent {
        /// Listen address for health endpoint
        #[arg(long, default_value = "0.0.0.0:9192")]
        listen: SocketAddr,
    },
}

#[derive(Subcommand, Debug)]
enum EbpfCommands {
    /// List BPF programs with cilium|netra|other classification
    Attachments {
        #[arg(short, long, default_value = "summary")]
        output: String,
    },
    /// Warn-only brotherhood drift findings
    Drift {
        #[arg(short, long, default_value = "summary")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Agent and status prefer quieter default logs unless verbose
    let quiet_cmds = matches!(
        args.command,
        None | Some(Commands::Status { .. })
            | Some(Commands::Info { .. })
            | Some(Commands::Version)
            | Some(Commands::Features { .. })
            | Some(Commands::Ebpf { .. })
    );
    if !quiet_cmds || args.verbose {
        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_target(false)
            .init();
    }

    match args.command {
        None => {
            cli::print_root_help();
            Ok(())
        }
        Some(Commands::Install {
            namespace,
            chart_directory,
            version,
            wait,
            set,
        }) => {
            cli::cmd_install(InstallOpts {
                namespace,
                chart_directory,
                version,
                wait,
                set,
            })
            .await
        }
        Some(Commands::Upgrade {
            namespace,
            chart_directory,
            wait,
            set,
        }) => {
            cli::cmd_upgrade(InstallOpts {
                namespace,
                chart_directory,
                version: None,
                wait,
                set,
            })
            .await
        }
        Some(Commands::Uninstall { namespace }) => cli::cmd_uninstall(&namespace).await,
        Some(Commands::Status {
            namespace,
            wait,
            output,
        }) => cli::cmd_status(&namespace, wait, &output).await,
        Some(Commands::Info { namespace }) => cli::cmd_info(&namespace).await,
        Some(Commands::Features { output }) => cli::cmd_features(&output),
        Some(Commands::Ebpf { command }) => match command {
            EbpfCommands::Attachments { output } => cli::cmd_ebpf_attachments(&output),
            EbpfCommands::Drift { output } => cli::cmd_ebpf_drift(&output),
        },
        Some(Commands::Version) => {
            println!("paqtra v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Commands::Tui {
            skip_bootstrap,
            hubble_port,
            auto_install,
            auto_upgrade,
        }) => run_tui(skip_bootstrap, hubble_port, auto_install, auto_upgrade).await,
        Some(Commands::Agent { listen }) => cli::run_agent(listen).await,
    }
}

async fn run_tui(
    skip_bootstrap: bool,
    hubble_port: u16,
    auto_install: bool,
    auto_upgrade: bool,
) -> Result<()> {
    let (context, hubble_port, k8s_client) = if skip_bootstrap {
        let k8s_client = kubernetes::K8sClient::new().await?;
        ("unknown".to_string(), hubble_port, k8s_client)
    } else {
        let bootstrap = BootstrapManager::new().await?;
        let k8s_client_clone = bootstrap.get_k8s_client();
        let result = bootstrap
            .run_bootstrap_with_options(auto_install, auto_upgrade)
            .await?;
        (result.context, result.hubble_port, k8s_client_clone)
    };

    let mut app = TuiApp::new(context, hubble_port, k8s_client).await?;
    app.run().await?;
    Ok(())
}
