use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use super::helm::{
    default_namespace, default_release, ensure_ok, helm, resolve_chart_dir,
};

pub struct InstallOpts {
    pub namespace: String,
    pub chart_directory: Option<PathBuf>,
    pub version: Option<String>,
    pub wait: bool,
    pub set: Vec<String>,
}

impl Default for InstallOpts {
    fn default() -> Self {
        Self {
            namespace: default_namespace(),
            chart_directory: None,
            version: None,
            wait: true,
            set: Vec::new(),
        }
    }
}

pub async fn cmd_install(opts: InstallOpts) -> Result<()> {
    let chart = resolve_chart_dir(opts.chart_directory.as_deref())?;
    let release = default_release();
    let ns = &opts.namespace;

    println!("{} Installing Paqtra into namespace {}...", "→".cyan(), ns.bold());
    println!("  Chart: {}", chart.display());

    let mut args: Vec<String> = vec![
        "upgrade".into(),
        "--install".into(),
        release.clone(),
        chart.display().to_string(),
        "--namespace".into(),
        ns.clone(),
        "--create-namespace".into(),
        "--set".into(),
        format!("global.namespace={ns}"),
    ];

    if let Some(ver) = &opts.version {
        args.push("--version".into());
        args.push(ver.clone());
    }
    for s in &opts.set {
        args.push("--set".into());
        args.push(s.clone());
    }
    if opts.wait {
        args.push("--wait".into());
        args.push("--timeout".into());
        args.push("5m".into());
    }

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = helm(&arg_refs).await?;
    ensure_ok(&out, "helm install")?;
    print!("{}", String::from_utf8_lossy(&out.stdout));
    println!("{} Paqtra installed. Run `{}` to check.", "✔".green(), "paqtra status".cyan());
    Ok(())
}

pub async fn cmd_upgrade(opts: InstallOpts) -> Result<()> {
    let chart = resolve_chart_dir(opts.chart_directory.as_deref())?;
    let release = default_release();
    let ns = &opts.namespace;

    println!("{} Upgrading Paqtra in {}...", "→".cyan(), ns.bold());

    let mut args: Vec<String> = vec![
        "upgrade".into(),
        release,
        chart.display().to_string(),
        "--namespace".into(),
        ns.clone(),
        "--reuse-values".into(),
    ];
    for s in &opts.set {
        args.push("--set".into());
        args.push(s.clone());
    }
    if opts.wait {
        args.push("--wait".into());
        args.push("--timeout".into());
        args.push("5m".into());
    }

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = helm(&arg_refs).await?;
    ensure_ok(&out, "helm upgrade")?;
    print!("{}", String::from_utf8_lossy(&out.stdout));
    println!("{} Upgrade complete.", "✔".green());
    Ok(())
}

pub async fn cmd_uninstall(namespace: &str) -> Result<()> {
    let release = default_release();
    println!("{} Uninstalling Paqtra from {}...", "→".cyan(), namespace.bold());
    let out = helm(&["uninstall", &release, "--namespace", namespace]).await?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("not found") || err.contains("release: not found") {
            println!("{} No release found (already uninstalled).", "⚠".yellow());
            return Ok(());
        }
        ensure_ok(&out, "helm uninstall")?;
    }
    print!("{}", String::from_utf8_lossy(&out.stdout));
    println!("{} Paqtra uninstalled.", "✔".green());
    Ok(())
}
