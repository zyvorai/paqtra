// Policy data model - canonical definition used across handlers and services
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub namespace: String,
    pub spec: Value,
}
