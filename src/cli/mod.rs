//! Cilium-style management CLI: banner, install, status, info, agent, features.

mod agent;
mod banner;
mod features;
mod helm;
mod info;
mod install;
mod status;

pub use agent::run_agent;
pub use banner::print_root_help;
pub use features::{cmd_ebpf_attachments, cmd_ebpf_drift, cmd_features};
pub use info::cmd_info;
pub use install::{cmd_install, cmd_uninstall, cmd_upgrade, InstallOpts};
pub use status::cmd_status;
