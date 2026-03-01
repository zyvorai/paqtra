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
