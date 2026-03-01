// Hubble service client
//
// This service wraps Hubble gRPC/CLI interactions for the web API.
// Currently a stub - implement actual Hubble integration to get real flow data.

pub struct HubbleService {
    address: String,
}

impl HubbleService {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Check if Hubble is reachable
    pub async fn is_healthy(&self) -> bool {
        // TODO: Implement actual health check against Hubble relay
        tokio::net::TcpStream::connect(&self.address).await.is_ok()
    }
}
