//! Native port-forwarding (no `kubectl port-forward`), used by
//! `paqtra hubble port-forward` and `paqtra ui`.

use anyhow::{bail, Context, Result};
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use kube::{Api, Client};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

/// The first Running pod matching `selector`.
pub async fn find_pod(client: &Client, ns: &str, selector: &str) -> Result<String> {
    let api: Api<Pod> = Api::namespaced(client.clone(), ns);
    let list = api
        .list(&ListParams::default().labels(selector))
        .await
        .with_context(|| format!("cannot list pods ({selector}) in {ns}"))?;
    list.items
        .into_iter()
        .find(|p| p.status.as_ref().and_then(|s| s.phase.as_deref()) == Some("Running"))
        .and_then(|p| p.metadata.name)
        .with_context(|| format!("no running pod with labels {selector} in namespace {ns}"))
}

async fn forward_one(api: Api<Pod>, pod: String, remote: u16, mut local: TcpStream) -> Result<()> {
    let mut pf = api
        .portforward(&pod, &[remote])
        .await
        .context("port-forward refused")?;
    let mut upstream = pf.take_stream(remote).context("no stream for the port")?;
    tokio::io::copy_bidirectional(&mut local, &mut upstream)
        .await
        .ok();
    drop(upstream);
    pf.join().await.ok();
    Ok(())
}

/// Listen on `bind` and forward every connection to `pod:remote` until Ctrl-C.
pub async fn serve(
    client: Client,
    ns: &str,
    pod: &str,
    remote: u16,
    bind: SocketAddr,
) -> Result<()> {
    let listener = TcpListener::bind(bind).await.with_context(|| {
        format!("cannot listen on {bind} (is the port already in use? try --port)")
    })?;
    let api: Api<Pod> = Api::namespaced(client, ns);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (sock, _) = accepted?;
                let (api, pod) = (api.clone(), pod.to_string());
                tokio::spawn(async move {
                    if let Err(e) = forward_one(api, pod, remote, sock).await {
                        eprintln!("port-forward connection failed: {e:#}");
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
    }
}

/// Open a URL in the user's browser; failure is only a hint, never an error.
pub fn open_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if std::process::Command::new(opener)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_err()
    {
        eprintln!("(could not launch a browser; open {url} yourself)");
    }
}

pub fn bail_if_privileged(port: u16) -> Result<()> {
    if port < 1024 && !cfg!(windows) {
        bail!("port {port} is privileged; pick one >= 1024 with --port");
    }
    Ok(())
}
