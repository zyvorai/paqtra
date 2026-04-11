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

/// Maximum serialized size for the spec field (100 KB).
const MAX_SPEC_SIZE: usize = 100 * 1024;
/// Maximum nesting depth for the spec JSON.
const MAX_SPEC_DEPTH: usize = 20;

impl CreatePolicyRequest {
    /// Validate the spec field: reject specs that are too large or too deeply nested.
    pub fn validate_spec(&self) -> Result<(), String> {
        let serialized = serde_json::to_string(&self.spec)
            .map_err(|e| format!("Failed to serialize spec: {}", e))?;
        if serialized.len() > MAX_SPEC_SIZE {
            return Err(format!(
                "Policy spec exceeds maximum size of {} bytes (actual: {} bytes)",
                MAX_SPEC_SIZE,
                serialized.len()
            ));
        }
        let depth = json_depth(&self.spec);
        if depth > MAX_SPEC_DEPTH {
            return Err(format!(
                "Policy spec exceeds maximum nesting depth of {} (actual: {})",
                MAX_SPEC_DEPTH, depth
            ));
        }
        Ok(())
    }
}

/// Compute the maximum nesting depth of a JSON value.
fn json_depth(value: &Value) -> usize {
    match value {
        Value::Array(arr) => 1 + arr.iter().map(json_depth).max().unwrap_or(0),
        Value::Object(map) => 1 + map.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}
