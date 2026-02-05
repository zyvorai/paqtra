use anyhow::Result;
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
                Err(_) => {
                    // Fallback to CLI
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

pub async fn start_port_forward() -> Result<u16> {
    tokio::spawn(async {
        let _ = Command::new("cilium")
            .args(["hubble", "port-forward"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    });

    // Give it time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(4245)
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
            .output()?;

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
            .output()?;

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
