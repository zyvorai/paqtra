use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;

use super::banner::print_banner;
use super::helm::{default_namespace, kubectl};
use crate::ebpf::attachments::{collect_inventory, drift_findings};

#[derive(Debug, Clone, Serialize)]
struct ComponentStatus {
    name: String,
    state: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct StatusReport {
    components: Vec<ComponentStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    drift: Vec<crate::ebpf::attachments::DriftFinding>,
    ok: bool,
}

fn row(name: &str, state: &str, detail: &str) {
    let lower = state.to_lowercase();
    let colored = if lower == "ok" || lower.contains("ready") {
        format!("{state:16}").green().to_string()
    } else if lower == "disabled" || lower == "n/a" || lower == "info" {
        format!("{state:16}").dimmed().to_string()
    } else if lower.contains("warning") || lower.contains("degraded") {
        format!("{state:16}").yellow().to_string()
    } else {
        format!("{state:16}").red().to_string()
    };
    println!("  {:18} {} {}", name, colored, detail.dimmed());
}

async fn pod_summary(ns: &str, label: &str) -> (String, String) {
    let out = kubectl(&[
        "get",
        "pods",
        "-n",
        ns,
        "-l",
        label,
        "-o",
        "jsonpath={range .items[*]}{.status.phase}{' '}{end}",
    ])
    .await;

    match out {
        Ok(o) if o.status.success() => {
            let phases = String::from_utf8_lossy(&o.stdout);
            let phases: Vec<&str> = phases.split_whitespace().collect();
            if phases.is_empty() {
                return ("Disabled".into(), "not deployed".into());
            }
            let ready = phases.iter().filter(|p| **p == "Running").count();
            let total = phases.len();
            if ready == total {
                (format!("{ready}/{total} Ready"), format!("{ready} pods running"))
            } else if ready > 0 {
                ("Degraded".into(), format!("{ready}/{total} running"))
            } else {
                ("Error".into(), format!("0/{total} running"))
            }
        }
        _ => ("Error".into(), "kubectl failed".into()),
    }
}

async fn ds_summary(ns: &str, name: &str) -> (String, String) {
    let out = kubectl(&[
        "get",
        "daemonset",
        name,
        "-n",
        ns,
        "-o",
        "jsonpath={.status.numberReady}/{.status.desiredNumberScheduled}",
    ])
    .await;

    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() || s == "/" {
                return ("Disabled".into(), "not deployed".into());
            }
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                let ready: u32 = parts[0].parse().unwrap_or(0);
                let desired: u32 = parts[1].parse().unwrap_or(0);
                if desired == 0 {
                    return ("Disabled".into(), "desired 0".into());
                }
                if ready == desired {
                    return ("OK".into(), format!("{ready}/{desired} nodes"));
                }
                if ready > 0 {
                    return ("Degraded".into(), format!("{ready}/{desired} nodes"));
                }
                return ("Error".into(), format!("{ready}/{desired} nodes"));
            }
            ("Warning".into(), s)
        }
        _ => ("Disabled".into(), "not deployed".into()),
    }
}

fn local_bpf_rows() -> (Vec<ComponentStatus>, Vec<crate::ebpf::attachments::DriftFinding>) {
    let inv = collect_inventory();
    let findings = drift_findings(&inv);
    let mut rows = vec![ComponentStatus {
        name: "BPF inventory".into(),
        state: if inv.source == "unavailable" {
            "Info".into()
        } else {
            "OK".into()
        },
        detail: format!(
            "{} via {} (cilium={} netra={} other={})",
            inv.total, inv.source, inv.cilium, inv.netra, inv.other
        ),
    }];
    for f in &findings {
        let state = if f.severity == "warning" {
            "Warning"
        } else {
            "Info"
        };
        rows.push(ComponentStatus {
            name: format!("Drift:{}", f.kind),
            state: state.into(),
            detail: f.message.clone(),
        });
    }
    (rows, findings)
}

pub async fn cmd_status(namespace: &str, wait: bool, output: &str) -> Result<()> {
    let ns = if namespace.is_empty() {
        default_namespace()
    } else {
        namespace.to_string()
    };

    if wait {
        println!("{} Waiting for Paqtra components...", "→".cyan());
        let _ = kubectl(&[
            "wait",
            "--for=condition=available",
            "deployment",
            "-l",
            "app.kubernetes.io/instance=paqtra",
            "-n",
            &ns,
            "--timeout=120s",
        ])
        .await;
    }

    let agent = ds_summary(&ns, "paqtra-agent").await;
    // fullname may be paqtra-agent when release is paqtra
    let agent = if agent.0 == "Disabled" {
        ds_summary(&ns, "paqtra-paqtra-agent").await
    } else {
        agent
    };
    // Also try label-based
    let agent = if agent.0 == "Disabled" {
        let (s, d) = pod_summary(&ns, "app.kubernetes.io/component=agent").await;
        if s == "Disabled" {
            agent
        } else {
            (s, d)
        }
    } else {
        agent
    };

    let api = pod_summary(&ns, "app.kubernetes.io/component=api").await;
    let ui = pod_summary(&ns, "app.kubernetes.io/component=ui").await;
    let cilium = pod_summary("kube-system", "k8s-app=cilium").await;
    let hubble = pod_summary("kube-system", "k8s-app=hubble-relay").await;

    let mut components = vec![
        ComponentStatus {
            name: "Agent".into(),
            state: agent.0.clone(),
            detail: agent.1.clone(),
        },
        ComponentStatus {
            name: "API".into(),
            state: api.0.clone(),
            detail: api.1.clone(),
        },
        ComponentStatus {
            name: "UI".into(),
            state: ui.0.clone(),
            detail: ui.1.clone(),
        },
        ComponentStatus {
            name: "Cilium".into(),
            state: cilium.0.clone(),
            detail: cilium.1.clone(),
        },
        ComponentStatus {
            name: "Hubble Relay".into(),
            state: hubble.0.clone(),
            detail: hubble.1.clone(),
        },
    ];

    let (bpf_rows, drift) = local_bpf_rows();
    components.extend(bpf_rows);

    let ok = components.iter().all(|c| {
        c.state.contains("Ready")
            || c.state == "OK"
            || c.state == "Disabled"
            || c.state == "Info"
            || c.state == "Warning"
            || c.name == "Cilium"
            || c.name == "Hubble Relay"
            || c.name.starts_with("Drift:")
            || c.name == "BPF inventory"
    }) && components.iter().any(|c| {
        (c.name == "Agent" || c.name == "API") && (c.state.contains("Ready") || c.state == "OK")
    });

    let report = StatusReport {
        components: components.clone(),
        drift,
        ok,
    };

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_banner();
    for c in &components {
        row(&c.name, &c.state, &c.detail);
    }
    println!();
    if ok {
        println!("{} Paqtra is running", "✔".green());
    } else {
        println!("{} Paqtra has issues — see above", "✖".red());
    }
    Ok(())
}
