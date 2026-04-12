// Flow data model - canonical definition used across handlers and services
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub id: String,
    pub timestamp: String,
    pub source: FlowEndpoint,
    pub destination: FlowEndpoint,
    pub verdict: String,
    pub protocol: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEndpoint {
    pub namespace: String,
    pub pod: String,
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct FlowQueryParams {
    pub namespace: Option<String>,
    pub verdict: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStats {
    pub total_flows: u64,
    pub forwarded: u64,
    pub dropped: u64,
    pub requests_per_second: f64,
    pub avg_latency_ms: f64,
}

impl Default for FlowStats {
    fn default() -> Self {
        Self {
            total_flows: 0,
            forwarded: 0,
            dropped: 0,
            requests_per_second: 0.0,
            avg_latency_ms: 0.0,
        }
    }
}
