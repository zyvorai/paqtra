use anyhow::{Context, Result};
use std::process::{Command, Stdio};

#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "grpc")]
pub enum HubbleClient {
    Grpc(grpc::GrpcHubbleClient),
    Cli(CliHubbleClient),
}

#[cfg(not(feature = "grpc"))]
pub type HubbleClient = CliHubbleClient;

#[cfg(feature = "grpc")]
impl HubbleClient {
    pub async fn new(port: u16, use_grpc: bool) -> Result<Self> {
        if use_grpc {
            match grpc::GrpcHubbleClient::connect(format!("http://localhost:{}", port)).await {
                Ok(client) => Ok(HubbleClient::Grpc(client)),
                Err(e) => {
                    tracing::warn!("gRPC connection failed, falling back to CLI: {}", e);
                    Ok(HubbleClient::Cli(CliHubbleClient::new(port)))
                }
            }
        } else {
            Ok(HubbleClient::Cli(CliHubbleClient::new(port)))
        }
    }

    pub async fn get_flows(&mut self) -> Result<Vec<Flow>> {
        match self {
            HubbleClient::Grpc(client) => client.get_flows(100).await,
            HubbleClient::Cli(client) => client.get_flows().await,
        }
    }
}

/// Start Hubble port-forward and verify it is running.
/// Returns the port on success, or an error if port-forward fails to start.
pub async fn start_port_forward() -> Result<u16> {
    let port: u16 = 4245;

    let child = Command::new("cilium")
        .args(["hubble", "port-forward"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start cilium hubble port-forward. Is cilium CLI installed?")?;

    // Store the child PID so we can clean up on exit
    let pid = child.id();
    tracing::info!("Started hubble port-forward (pid: {})", pid);

    // Register a cleanup handler for the port-forward process
    let pid_for_cleanup = pid;
    tokio::spawn(async move {
        // Wait for a shutdown signal or the process to exit
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Cleaning up port-forward process (pid: {})", pid_for_cleanup);
        #[cfg(unix)]
        {
            unsafe {
                let ret = libc::kill(pid_for_cleanup as i32, libc::SIGTERM);
                if ret != 0 {
                    tracing::warn!("Failed to send SIGTERM to pid {}: errno {}", pid_for_cleanup, std::io::Error::last_os_error());
                }
            }
        }
    });

    // Wait and verify the port-forward is actually listening
    let max_retries = 10;
    for attempt in 1..=max_retries {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Try connecting to verify port-forward is ready
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            Ok(_) => {
                tracing::info!("Hubble port-forward ready on port {}", port);
                return Ok(port);
            }
            Err(_) if attempt < max_retries => {
                tracing::debug!("Port-forward not ready yet (attempt {}/{})", attempt, max_retries);
            }
            Err(e) => {
                tracing::error!("Port-forward failed to start after {} attempts: {}", max_retries, e);
                // Kill the process since it's not working
                #[cfg(unix)]
                unsafe {
                    let ret = libc::kill(pid as i32, libc::SIGTERM);
                    if ret != 0 {
                        tracing::warn!("Failed to send SIGTERM to pid {}: errno {}", pid, std::io::Error::last_os_error());
                    }
                }
                anyhow::bail!(
                    "Hubble port-forward failed to start on port {}. \
                     Ensure Hubble is enabled and the Hubble relay is running.",
                    port
                );
            }
        }
    }

    Ok(port)
}

pub struct CliHubbleClient {
    _port: u16,
}

#[cfg(feature = "grpc")]
impl CliHubbleClient {
    pub fn new(port: u16) -> Self {
        Self { _port: port }
    }

    pub async fn get_flows(&self) -> Result<Vec<Flow>> {
        let output = Command::new("cilium")
            .args(["hubble", "observe", "--last", "100", "-o", "json"])
            .output()
            .context("Failed to execute cilium hubble observe")?;

        let flows_json = String::from_utf8_lossy(&output.stdout);
        let flows: Vec<Flow> = flows_json
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(flows)
    }
}

#[cfg(not(feature = "grpc"))]
impl CliHubbleClient {
    pub async fn new(port: u16, _use_grpc: bool) -> Result<Self> {
        Ok(CliHubbleClient { _port: port })
    }

    pub async fn get_flows(&self) -> Result<Vec<Flow>> {
        let output = Command::new("cilium")
            .args(["hubble", "observe", "--last", "100", "-o", "json"])
            .output()
            .context("Failed to execute cilium hubble observe")?;

        let flows_json = String::from_utf8_lossy(&output.stdout);
        let flows: Vec<Flow> = flows_json
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(flows)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Flow {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub source: FlowEndpoint,
    #[serde(default)]
    pub destination: FlowEndpoint,
    #[serde(default)]
    pub r#type: String,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct FlowEndpoint {
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub pod_name: String,
    #[serde(default)]
    pub ip: String,
}
