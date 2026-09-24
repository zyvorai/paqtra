use anyhow::Result;
use owo_colors::OwoColorize;

use super::helm::{default_namespace, default_release, kubectl};

pub async fn cmd_info(namespace: &str) -> Result<()> {
    let ns = if namespace.is_empty() {
        default_namespace()
    } else {
        namespace.to_string()
    };
    let release = default_release();

    println!("{}", "Paqtra Info".bold());
    println!();
    println!(
        "  {:20} {}",
        "CLI Version:".dimmed(),
        env!("CARGO_PKG_VERSION")
    );
    println!("  {:20} {}", "Release:".dimmed(), release);
    println!("  {:20} {}", "Namespace:".dimmed(), ns);

    let ctx = kubectl(&["config", "current-context"]).await?;
    if ctx.status.success() {
        println!(
            "  {:20} {}",
            "Kube Context:".dimmed(),
            String::from_utf8_lossy(&ctx.stdout).trim()
        );
    }

    let hubble = kubectl(&[
        "get",
        "configmap",
        &format!("{release}-config"),
        "-n",
        &ns,
        "-o",
        "jsonpath={.data.HUBBLE_ADDRESS}",
    ])
    .await;
    if let Ok(o) = hubble {
        if o.status.success() {
            let addr = String::from_utf8_lossy(&o.stdout);
            if !addr.is_empty() {
                println!("  {:20} {}", "Hubble:".dimmed(), addr);
            }
        }
    }

    let nodes = kubectl(&["get", "nodes", "--no-headers"]).await?;
    if nodes.status.success() {
        let n = String::from_utf8_lossy(&nodes.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count();
        println!("  {:20} {}", "Cluster Nodes:".dimmed(), n);
    }

    let agents = kubectl(&[
        "get",
        "pods",
        "-n",
        &ns,
        "-l",
        "app.kubernetes.io/component=agent",
        "--no-headers",
    ])
    .await?;
    if agents.status.success() {
        let n = String::from_utf8_lossy(&agents.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count();
        println!("  {:20} {}", "Agent Pods:".dimmed(), n);
    }

    println!();
    Ok(())
}
