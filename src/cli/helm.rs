use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn default_namespace() -> String {
    std::env::var("PAQTRA_NAMESPACE").unwrap_or_else(|_| "paqtra".to_string())
}

pub fn default_release() -> String {
    std::env::var("PAQTRA_RELEASE").unwrap_or_else(|_| "paqtra".to_string())
}

/// Resolve Helm chart directory: --chart-directory, env, cwd/chart, beside binary.
pub fn resolve_chart_dir(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if p.join("Chart.yaml").is_file() {
            return Ok(p.to_path_buf());
        }
        bail!("Chart.yaml not found in {}", p.display());
    }
    if let Ok(env) = std::env::var("PAQTRA_CHART_DIR") {
        let p = PathBuf::from(env);
        if p.join("Chart.yaml").is_file() {
            return Ok(p);
        }
    }
    let candidates = [
        PathBuf::from("chart"),
        PathBuf::from("./chart"),
        PathBuf::from("/usr/share/paqtra/chart"),
        PathBuf::from("/opt/paqtra/chart"),
    ];
    for c in &candidates {
        if c.join("Chart.yaml").is_file() {
            return Ok(c.clone());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for up in [dir.to_path_buf(), dir.join("../.."), dir.join("../../..")] {
                let chart = up.join("chart");
                if chart.join("Chart.yaml").is_file() {
                    return Ok(chart.canonicalize().unwrap_or(chart));
                }
            }
        }
    }
    bail!(
        "Helm chart not found. Pass --chart-directory or set PAQTRA_CHART_DIR, \
         or run from the repo root (./chart)."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_chart_dir_explicit_ok() {
        let dir = std::env::temp_dir().join(format!("paqtra-chart-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Chart.yaml"), "name: paqtra\n").unwrap();
        let got = resolve_chart_dir(Some(&dir)).unwrap();
        assert_eq!(got, dir);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_chart_dir_explicit_missing() {
        let dir = std::env::temp_dir().join(format!("paqtra-chart-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(resolve_chart_dir(Some(&dir)).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_namespace_and_release() {
        assert!(!default_namespace().is_empty());
        assert!(!default_release().is_empty());
    }
}

pub async fn helm(args: &[&str]) -> Result<std::process::Output> {
    Command::new("helm")
        .args(args)
        .output()
        .await
        .context("Failed to run helm — is Helm installed?")
}

pub async fn kubectl(args: &[&str]) -> Result<std::process::Output> {
    Command::new("kubectl")
        .args(args)
        .output()
        .await
        .context("Failed to run kubectl — is kubectl configured?")
}

pub fn ensure_ok(output: &std::process::Output, action: &str) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    bail!("{action} failed:\n{stdout}{stderr}");
}
