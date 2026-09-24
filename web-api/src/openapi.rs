// OpenAPI 3.0 specification and Swagger UI handler
use axum::{http::header, response::IntoResponse, Json};

/// Returns the OpenAPI 3.0 JSON specification.
pub async fn openapi_json() -> Json<serde_json::Value> {
    Json(openapi_spec())
}

/// Returns a minimal HTML page that loads Swagger UI from CDN.
pub async fn swagger_ui() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <title>Paqtra API</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css"/>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: '/api-docs/openapi.json',
      dom_id: '#swagger-ui',
      deepLinking: true,
      presets: [SwaggerUIBundle.presets.apis],
      layout: 'BaseLayout'
    });
  </script>
</body>
</html>"#;

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Build the complete OpenAPI 3.0 specification programmatically.
pub fn openapi_spec() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Paqtra API",
            "version": "1.0.0",
            "description": "Paqtra — trace every flow. Network observability and security for Cilium-powered Kubernetes clusters. Provides real-time flow monitoring, policy management, anomaly detection, compliance auditing, eBPF kernel introspection, and multi-cluster operations.",
            "contact": {
                "name": "Paqtra Team"
            },
            "license": {
                "name": "Apache 2.0",
                "url": "https://www.apache.org/licenses/LICENSE-2.0"
            }
        },
        "servers": [
            { "url": "/api/v1", "description": "Default API server" }
        ],
        "tags": [
            { "name": "Flows", "description": "Network flow monitoring and statistics" },
            { "name": "Policies", "description": "Cilium network policy management" },
            { "name": "Anomalies", "description": "Anomaly detection and remediation" },
            { "name": "Compliance", "description": "Security compliance frameworks and auditing" },
            { "name": "Security", "description": "Security posture and zero-trust scoring" },
            { "name": "Nodes & Endpoints", "description": "Kubernetes nodes, endpoints, and events" },
            { "name": "Modules", "description": "Intelligence modules: autopolicy, chaos, canary, replay, healer, rootcause" },
            { "name": "Diagnostics", "description": "Cluster diagnostics and connectivity testing" },
            { "name": "Infrastructure", "description": "Cluster health, node drain, service map, and DNS" },
            { "name": "eBPF", "description": "eBPF kernel map introspection: conntrack, ipcache, LB, drops" },
            { "name": "WebSocket", "description": "Real-time streaming via WebSocket" },
            { "name": "Health", "description": "Health and readiness probes" }
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT",
                    "description": "JWT token obtained from authentication. Pass as `Authorization: Bearer <token>`. Auth can be disabled in dev mode."
                }
            },
            "schemas": schemas(),
            "parameters": common_parameters(),
            "responses": common_responses()
        },
        "security": [{ "bearerAuth": [] }],
        "paths": paths()
    })
}

fn schemas() -> serde_json::Value {
    serde_json::json!({
        "Flow": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "example": "flow-abc123" },
                "timestamp": { "type": "string", "format": "date-time" },
                "source": { "$ref": "#/components/schemas/FlowEndpoint" },
                "destination": { "$ref": "#/components/schemas/FlowEndpoint" },
                "verdict": { "type": "string", "enum": ["FORWARDED", "DROPPED", "AUDIT", "REDIRECTED", "ERROR"] },
                "protocol": { "type": "string", "example": "TCP" },
                "port": { "type": "integer", "format": "uint16", "example": 80 },
                "http_method": { "type": "string", "nullable": true, "example": "GET" },
                "http_url": { "type": "string", "nullable": true, "example": "/api/health" },
                "http_code": { "type": "integer", "nullable": true, "example": 200 },
                "cluster": { "type": "string", "nullable": true, "example": "us-east-1" }
            },
            "required": ["id", "timestamp", "source", "destination", "verdict", "protocol", "port"]
        },
        "FlowEndpoint": {
            "type": "object",
            "properties": {
                "namespace": { "type": "string", "example": "default" },
                "pod": { "type": "string", "example": "nginx-7d4b8c5f-abc12" },
                "ip": { "type": "string", "example": "10.0.1.42" }
            },
            "required": ["namespace", "pod", "ip"]
        },
        "FlowStats": {
            "type": "object",
            "properties": {
                "total_flows": { "type": "integer", "format": "uint64" },
                "forwarded": { "type": "integer", "format": "uint64" },
                "dropped": { "type": "integer", "format": "uint64" },
                "requests_per_second": { "type": "number", "format": "double" },
                "avg_latency_ms": { "type": "number", "format": "double" }
            }
        },
        "Policy": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "example": "pol-abc123" },
                "name": { "type": "string", "example": "allow-frontend-to-backend" },
                "namespace": { "type": "string", "example": "default" },
                "created_at": { "type": "string", "format": "date-time" },
                "status": { "type": "string", "enum": ["active", "pending", "disabled", "error"] }
            },
            "required": ["id", "name", "namespace", "created_at", "status"]
        },
        "CreatePolicyRequest": {
            "type": "object",
            "properties": {
                "name": { "type": "string", "example": "allow-frontend-to-backend" },
                "namespace": { "type": "string", "example": "default" },
                "spec": {
                    "type": "object",
                    "description": "Cilium network policy specification (max 100KB, max depth 20)",
                    "additionalProperties": true
                }
            },
            "required": ["name", "namespace", "spec"]
        },
        "Anomaly": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "example": "anom-xyz789" },
                "type": { "type": "string", "example": "traffic_spike" },
                "severity": { "type": "string", "enum": ["critical", "high", "medium", "low", "info"] },
                "source": { "type": "string" },
                "destination": { "type": "string" },
                "namespace": { "type": "string" },
                "description": { "type": "string" },
                "detected_at": { "type": "string", "format": "date-time" },
                "status": { "type": "string", "enum": ["active", "acknowledged", "resolved"] }
            }
        },
        "ComplianceFramework": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "example": "cis-k8s-1.8" },
                "name": { "type": "string", "example": "CIS Kubernetes Benchmark 1.8" },
                "version": { "type": "string" },
                "description": { "type": "string" },
                "total_controls": { "type": "integer" },
                "passing": { "type": "integer" },
                "failing": { "type": "integer" },
                "not_applicable": { "type": "integer" }
            }
        },
        "AuditResult": {
            "type": "object",
            "properties": {
                "framework": { "type": "string" },
                "timestamp": { "type": "string", "format": "date-time" },
                "overall_score": { "type": "number", "format": "double" },
                "total_controls": { "type": "integer" },
                "passing": { "type": "integer" },
                "failing": { "type": "integer" },
                "findings": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/AuditFinding" }
                }
            }
        },
        "AuditFinding": {
            "type": "object",
            "properties": {
                "control_id": { "type": "string" },
                "title": { "type": "string" },
                "status": { "type": "string", "enum": ["pass", "fail", "not_applicable"] },
                "severity": { "type": "string" },
                "description": { "type": "string" }
            }
        },
        "K8sNode": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "status": { "type": "string" },
                "roles": { "type": "array", "items": { "type": "string" } },
                "kubernetes_version": { "type": "string" },
                "os": { "type": "string" },
                "cpu_capacity": { "type": "string" },
                "memory_capacity": { "type": "string" },
                "pod_cidr": { "type": "string" },
                "internal_ip": { "type": "string" },
                "cilium_version": { "type": "string" }
            }
        },
        "CiliumEndpoint": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "namespace": { "type": "string" },
                "identity": { "type": "integer" },
                "ip": { "type": "string" },
                "status": { "type": "string" },
                "node": { "type": "string" },
                "labels": { "type": "object", "additionalProperties": { "type": "string" } }
            }
        },
        "K8sEvent": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "type": { "type": "string" },
                "reason": { "type": "string" },
                "message": { "type": "string" },
                "namespace": { "type": "string" },
                "involved_object": { "type": "string" },
                "timestamp": { "type": "string", "format": "date-time" },
                "count": { "type": "integer" }
            }
        },
        "SecurityFinding": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "severity": { "type": "string", "enum": ["critical", "high", "medium", "low"] },
                "category": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "resource": { "type": "string" },
                "namespace": { "type": "string" },
                "remediation": { "type": "string" }
            }
        },
        "SecurityPosture": {
            "type": "object",
            "properties": {
                "overall_score": { "type": "number", "format": "double", "example": 78.5 },
                "risk_level": { "type": "string", "enum": ["critical", "high", "medium", "low"] },
                "total_findings": { "type": "integer" },
                "categories": { "type": "object", "additionalProperties": { "type": "integer" } }
            }
        },
        "ZeroTrustScore": {
            "type": "object",
            "properties": {
                "overall_score": { "type": "number", "format": "double" },
                "identity_score": { "type": "number", "format": "double" },
                "network_score": { "type": "number", "format": "double" },
                "workload_score": { "type": "number", "format": "double" },
                "encryption_score": { "type": "number", "format": "double" }
            }
        },
        "ConntrackEntry": {
            "type": "object",
            "properties": {
                "src_ip": { "type": "string" },
                "dst_ip": { "type": "string" },
                "src_port": { "type": "integer" },
                "dst_port": { "type": "integer" },
                "protocol": { "type": "string" },
                "state": { "type": "string" },
                "lifetime": { "type": "string" },
                "rx_packets": { "type": "integer" },
                "tx_packets": { "type": "integer" }
            }
        },
        "IpcacheEntry": {
            "type": "object",
            "properties": {
                "cidr": { "type": "string" },
                "identity": { "type": "integer" },
                "tunnel_endpoint": { "type": "string", "nullable": true },
                "host_ip": { "type": "string", "nullable": true }
            }
        },
        "LbBackend": {
            "type": "object",
            "properties": {
                "service_id": { "type": "integer" },
                "frontend": { "type": "string" },
                "backend_id": { "type": "integer" },
                "backend_addr": { "type": "string" },
                "protocol": { "type": "string" },
                "weight": { "type": "integer" }
            }
        },
        "DropStats": {
            "type": "object",
            "properties": {
                "reason": { "type": "string" },
                "count": { "type": "integer" },
                "direction": { "type": "string" },
                "last_seen": { "type": "string", "format": "date-time" }
            }
        },
        "ErrorResponse": {
            "type": "object",
            "properties": {
                "error": { "type": "string", "description": "Human-readable error message" }
            },
            "required": ["error"]
        },
        "PaginatedResponse": {
            "type": "object",
            "properties": {
                "total": { "type": "integer", "description": "Total number of items" },
                "limit": { "type": "integer", "description": "Page size applied" },
                "offset": { "type": "integer", "description": "Starting offset" }
            },
            "description": "All list endpoints return pagination metadata alongside items"
        },
        "ClusterHealth": {
            "type": "object",
            "properties": {
                "status": { "type": "string", "enum": ["healthy", "degraded", "unhealthy"] },
                "nodes_ready": { "type": "integer" },
                "nodes_total": { "type": "integer" },
                "cilium_agents_ready": { "type": "integer" },
                "cilium_agents_total": { "type": "integer" }
            }
        },
        "DiagnosticsResult": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "timestamp": { "type": "string", "format": "date-time" },
                "checks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "status": { "type": "string", "enum": ["pass", "fail", "warn"] },
                            "message": { "type": "string" }
                        }
                    }
                }
            }
        }
    })
}

fn common_parameters() -> serde_json::Value {
    serde_json::json!({
        "limitParam": {
            "name": "limit",
            "in": "query",
            "description": "Maximum number of items to return (default 50, max 1000)",
            "required": false,
            "schema": { "type": "integer", "default": 50, "maximum": 1000 }
        },
        "offsetParam": {
            "name": "offset",
            "in": "query",
            "description": "Number of items to skip for pagination",
            "required": false,
            "schema": { "type": "integer", "default": 0 }
        },
        "namespaceParam": {
            "name": "namespace",
            "in": "query",
            "description": "Filter by Kubernetes namespace",
            "required": false,
            "schema": { "type": "string" }
        }
    })
}

fn common_responses() -> serde_json::Value {
    serde_json::json!({
        "Unauthorized": {
            "description": "Missing or invalid JWT token",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" },
                    "example": { "error": "Missing or invalid Authorization header" }
                }
            }
        },
        "Forbidden": {
            "description": "Insufficient permissions (admin role required)",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" },
                    "example": { "error": "Admin role required" }
                }
            }
        },
        "NotFound": {
            "description": "Resource not found",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" },
                    "example": { "error": "Resource not found" }
                }
            }
        },
        "InternalError": {
            "description": "Internal server error",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ErrorResponse" },
                    "example": { "error": "Internal server error" }
                }
            }
        }
    })
}

fn paths() -> serde_json::Value {
    let mut all = serde_json::Map::new();
    for part in [
        paths_health_and_flows(),
        paths_policies(),
        paths_anomalies_compliance_security(),
        paths_nodes_endpoints_modules(),
        paths_diagnostics_infra_ebpf_ws(),
    ] {
        if let serde_json::Value::Object(map) = part {
            all.extend(map);
        }
    }
    serde_json::Value::Object(all)
}

fn paths_health_and_flows() -> serde_json::Value {
    serde_json::json!({
        "/health": {
            "get": {
                "tags": ["Health"],
                "summary": "Health check",
                "description": "Returns server health status. No authentication required.",
                "security": [],
                "responses": {
                    "200": {
                        "description": "Server is healthy",
                        "content": { "application/json": { "schema": { "type": "object", "properties": { "status": { "type": "string" } } } } }
                    }
                }
            }
        },
        "/ready": {
            "get": {
                "tags": ["Health"],
                "summary": "Readiness probe",
                "description": "Returns whether the server is ready to accept traffic. No authentication required.",
                "security": [],
                "responses": {
                    "200": { "description": "Server is ready" },
                    "503": { "description": "Server is not ready" }
                }
            }
        },

        // ---- Flows ----
        "/flows": {
            "get": {
                "tags": ["Flows"],
                "summary": "List network flows",
                "description": "Retrieve network flows from Hubble relay with optional filtering by namespace, verdict, and pagination.",
                "parameters": [
                    { "$ref": "#/components/parameters/namespaceParam" },
                    { "name": "verdict", "in": "query", "description": "Filter by verdict (FORWARDED, DROPPED, etc.)", "schema": { "type": "string" } },
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" }
                ],
                "responses": {
                    "200": {
                        "description": "List of flows with pagination metadata",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "flows": { "type": "array", "items": { "$ref": "#/components/schemas/Flow" } },
                                "total": { "type": "integer" },
                                "limit": { "type": "integer" },
                                "offset": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/flows/{id}": {
            "get": {
                "tags": ["Flows"],
                "summary": "Get flow by ID",
                "description": "Retrieve a single network flow by its unique identifier.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": {
                        "description": "Flow details",
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Flow" } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/flows/stats": {
            "get": {
                "tags": ["Flows"],
                "summary": "Flow statistics",
                "description": "Get aggregate flow statistics including total counts, forwarded/dropped breakdown, RPS, and average latency.",
                "responses": {
                    "200": {
                        "description": "Flow statistics",
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/FlowStats" } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        }
    })
}

fn paths_policies() -> serde_json::Value {
    serde_json::json!({
        // ---- Policies ----
        "/policies": {
            "get": {
                "tags": ["Policies"],
                "summary": "List network policies",
                "description": "Retrieve all Cilium network policies with optional namespace filtering and pagination.",
                "parameters": [
                    { "$ref": "#/components/parameters/namespaceParam" },
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" }
                ],
                "responses": {
                    "200": {
                        "description": "List of policies",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "policies": { "type": "array", "items": { "$ref": "#/components/schemas/Policy" } },
                                "total": { "type": "integer" },
                                "limit": { "type": "integer" },
                                "offset": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            },
            "post": {
                "tags": ["Policies"],
                "summary": "Create network policy",
                "description": "Create a new Cilium network policy. Requires admin role. Spec is validated for size (max 100KB) and depth (max 20 levels).",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreatePolicyRequest" } } }
                },
                "responses": {
                    "201": {
                        "description": "Policy created",
                        "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Policy" } } }
                    },
                    "400": { "description": "Invalid request body or policy spec", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/policies/{id}": {
            "get": {
                "tags": ["Policies"],
                "summary": "Get policy by ID",
                "description": "Retrieve a single network policy by its unique identifier.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Policy details", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Policy" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            },
            "put": {
                "tags": ["Policies"],
                "summary": "Update policy",
                "description": "Update an existing network policy. Requires admin role.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreatePolicyRequest" } } }
                },
                "responses": {
                    "200": { "description": "Policy updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Policy" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            },
            "delete": {
                "tags": ["Policies"],
                "summary": "Delete policy",
                "description": "Delete a network policy. Requires admin role.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Policy deleted" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/policies/simulate": {
            "post": {
                "tags": ["Policies"],
                "summary": "Evidence-backed policy preview",
                "description": "Preview proposed CiliumNetworkPolicy impact against indexed flows and resolved selectors. Unsupported constructs (FQDN, L7, deny precedence) return unknown — never a confident pass. Each claim includes confidence: observed|inferred|unavailable.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreatePolicyRequest" } } }
                },
                "responses": {
                    "200": { "description": "Preview with pairs, uncertainty, and CRD rollback plan" },
                    "400": { "description": "Invalid request", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorResponse" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/investigate/path": {
            "post": {
                "tags": ["Investigate"],
                "summary": "Why can’t A reach B?",
                "description": "Join flows, Kubernetes Services/endpoints, CNP presence, DNS signals, node-path hints, and change events. Every step is labeled observed|inferred|unavailable.",
                "responses": {
                    "200": { "description": "Investigation result with steps, owner, and next actions" },
                    "400": { "description": "Invalid request" },
                    "401": { "$ref": "#/components/responses/Unauthorized" }
                }
            }
        },
        "/investigate/bundles/{id}": {
            "get": {
                "tags": ["Investigate"],
                "summary": "Fetch redacted investigation evidence bundle",
                "description": "Returns flows, policy names, steps, and timestamps. No payloads or Secret contents.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Evidence bundle" },
                    "404": { "description": "Bundle not found" },
                    "401": { "$ref": "#/components/responses/Unauthorized" }
                }
            }
        },
        "/policies/validate": {
            "post": {
                "tags": ["Policies"],
                "summary": "Validate policy YAML/JSON",
                "description": "Validate a Cilium network policy specification without creating it.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CreatePolicyRequest" } } }
                },
                "responses": {
                    "200": { "description": "Validation result", "content": { "application/json": { "schema": { "type": "object", "properties": { "valid": { "type": "boolean" }, "errors": { "type": "array", "items": { "type": "string" } } } } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        }
    })
}

fn paths_anomalies_compliance_security() -> serde_json::Value {
    serde_json::json!({
        // ---- Anomalies ----
        "/anomalies": {
            "get": {
                "tags": ["Anomalies"],
                "summary": "List detected anomalies",
                "description": "Retrieve all detected network anomalies with optional severity and status filtering.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" },
                    { "name": "severity", "in": "query", "schema": { "type": "string" }, "description": "Filter by severity" },
                    { "name": "status", "in": "query", "schema": { "type": "string" }, "description": "Filter by status" }
                ],
                "responses": {
                    "200": {
                        "description": "List of anomalies",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "anomalies": { "type": "array", "items": { "$ref": "#/components/schemas/Anomaly" } },
                                "total": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/anomalies/{id}": {
            "get": {
                "tags": ["Anomalies"],
                "summary": "Get anomaly by ID",
                "description": "Retrieve details of a specific detected anomaly.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Anomaly details", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Anomaly" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/anomalies/{id}/remediate": {
            "post": {
                "tags": ["Anomalies"],
                "summary": "Remediate anomaly",
                "description": "Apply automated remediation for a detected anomaly. Requires admin role.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Remediation applied" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },

        // ---- Compliance ----
        "/compliance/frameworks": {
            "get": {
                "tags": ["Compliance"],
                "summary": "List compliance frameworks",
                "description": "Retrieve available compliance frameworks (CIS, SOC2, PCI-DSS, HIPAA, NIST) with current pass/fail counts.",
                "responses": {
                    "200": {
                        "description": "List of compliance frameworks",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "frameworks": { "type": "array", "items": { "$ref": "#/components/schemas/ComplianceFramework" } }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/compliance/audit": {
            "post": {
                "tags": ["Compliance"],
                "summary": "Run network security checks",
                "description": "Run Paqtra's automated network-layer checks (default-deny policy coverage, Hubble reachability, transparent encryption, dropped flows) and store the result. The checks are not mapped to individual controls of the framework: the framework is context only, and the result is not a compliance assessment. A check whose input could not be read is reported as skipped, never passed. Requires the editor or admin role.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "framework": { "type": "string", "description": "A framework id from GET /compliance/frameworks. Unknown values are rejected with 400.", "example": "pci-dss-4.0", "default": "pci-dss-4.0" }
                        }
                    } } }
                },
                "responses": {
                    "200": { "description": "Stored audit result", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/AuditResult" } } } },
                    "400": { "description": "Unknown framework" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "description": "Editor or admin role required" }
                }
            }
        },
        "/compliance/audits": {
            "get": {
                "tags": ["Compliance"],
                "summary": "List stored audits",
                "description": "Stored audits, newest first (up to 100), as summaries without findings. Not available to namespace-limited accounts.",
                "responses": {
                    "200": { "description": "Audit summaries", "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "audits": { "type": "array", "items": { "type": "object" } },
                            "total": { "type": "integer" }
                        }
                    } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" }
                }
            }
        },
        "/compliance/audits/{id}": {
            "get": {
                "tags": ["Compliance"],
                "summary": "Get a stored audit",
                "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
                "responses": {
                    "200": { "description": "The audit with its findings", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/AuditResult" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "description": "Audit not found" }
                }
            }
        },
        "/compliance/audits/{id}/report": {
            "get": {
                "tags": ["Compliance"],
                "summary": "Export an audit report",
                "description": "Render a stored audit as HTML (default; print to PDF from the browser), CSV or JSON. The HTML report escapes every value, carries a Content-Security-Policy that forbids scripts, and states that it is not a compliance assessment. Each export is recorded in the audit log.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } },
                    { "name": "format", "in": "query", "required": false, "schema": { "type": "string", "enum": ["html", "csv", "json"], "default": "html" } }
                ],
                "responses": {
                    "200": { "description": "The report", "content": { "text/html": {}, "text/csv": {}, "application/json": {} } },
                    "400": { "description": "Unsupported format" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "description": "Audit not found" }
                }
            }
        },

        // ---- Security ----
        "/security/posture": {
            "get": {
                "tags": ["Security"],
                "summary": "Security posture overview",
                "description": "Get the overall security posture score with category breakdown.",
                "responses": {
                    "200": { "description": "Security posture", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SecurityPosture" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/security/findings": {
            "get": {
                "tags": ["Security"],
                "summary": "List security findings",
                "description": "Retrieve security findings with optional severity and category filtering.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" },
                    { "name": "severity", "in": "query", "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": {
                        "description": "Security findings",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "findings": { "type": "array", "items": { "$ref": "#/components/schemas/SecurityFinding" } },
                                "total": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/security/zero-trust": {
            "get": {
                "tags": ["Security"],
                "summary": "Zero-trust maturity score",
                "description": "Get a zero-trust maturity assessment across identity, network, workload, and encryption dimensions.",
                "responses": {
                    "200": { "description": "Zero-trust scores", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ZeroTrustScore" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        }
    })
}

fn paths_nodes_endpoints_modules() -> serde_json::Value {
    serde_json::json!({
        // ---- Nodes & Endpoints ----
        "/nodes": {
            "get": {
                "tags": ["Nodes & Endpoints"],
                "summary": "List Kubernetes nodes",
                "description": "Retrieve all cluster nodes with Cilium agent status, versions, and capacity.",
                "responses": {
                    "200": { "description": "List of nodes", "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "nodes": { "type": "array", "items": { "$ref": "#/components/schemas/K8sNode" } },
                            "total": { "type": "integer" }
                        }
                    } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/endpoints": {
            "get": {
                "tags": ["Nodes & Endpoints"],
                "summary": "List Cilium endpoints",
                "description": "Retrieve all Cilium-managed endpoints with identity, IP, and policy status.",
                "parameters": [
                    { "$ref": "#/components/parameters/namespaceParam" },
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" }
                ],
                "responses": {
                    "200": { "description": "List of endpoints", "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "endpoints": { "type": "array", "items": { "$ref": "#/components/schemas/CiliumEndpoint" } },
                            "total": { "type": "integer" }
                        }
                    } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/events": {
            "get": {
                "tags": ["Nodes & Endpoints"],
                "summary": "List Kubernetes events",
                "description": "Retrieve recent Kubernetes events with optional namespace filtering.",
                "parameters": [
                    { "$ref": "#/components/parameters/namespaceParam" },
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" }
                ],
                "responses": {
                    "200": { "description": "List of events", "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "events": { "type": "array", "items": { "$ref": "#/components/schemas/K8sEvent" } },
                            "total": { "type": "integer" }
                        }
                    } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },

        // ---- Modules ----
        "/modules/autopolicy/generate": {
            "post": {
                "tags": ["Modules"],
                "summary": "Generate auto-policy",
                "description": "Automatically generate a Cilium network policy based on observed traffic patterns.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "namespace": { "type": "string" },
                            "workload": { "type": "string" },
                            "duration": { "type": "string", "description": "Observation window (e.g. '1h', '24h')" }
                        },
                        "required": ["namespace"]
                    } } }
                },
                "responses": {
                    "200": { "description": "Generated policy" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/chaos/experiments": {
            "get": {
                "tags": ["Modules"],
                "summary": "List chaos experiments",
                "description": "Retrieve all chaos engineering experiments and their results.",
                "responses": {
                    "200": { "description": "List of chaos experiments" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/chaos/run": {
            "post": {
                "tags": ["Modules"],
                "summary": "Run chaos experiment",
                "description": "Launch a chaos engineering experiment (packet loss, latency injection, etc.). Requires admin role.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string", "enum": ["packet_loss", "latency", "bandwidth", "dns_failure"] },
                            "namespace": { "type": "string" },
                            "target": { "type": "string" },
                            "duration_seconds": { "type": "integer" },
                            "parameters": { "type": "object", "additionalProperties": true }
                        },
                        "required": ["type", "namespace", "target"]
                    } } }
                },
                "responses": {
                    "200": { "description": "Experiment started" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/canary/{id}": {
            "get": {
                "tags": ["Modules"],
                "summary": "Get canary deployment status",
                "description": "Check the status and metrics of a canary deployment.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Canary status" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/replay/recordings": {
            "get": {
                "tags": ["Modules"],
                "summary": "List replay recordings",
                "description": "List all flow replay recording sessions.",
                "responses": {
                    "200": { "description": "List of recordings" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/replay/start": {
            "post": {
                "tags": ["Modules"],
                "summary": "Start recording",
                "description": "Start a new flow recording session for later replay.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "namespace": { "type": "string" },
                            "duration_seconds": { "type": "integer" },
                            "filters": { "type": "object", "additionalProperties": true }
                        }
                    } } }
                },
                "responses": {
                    "200": { "description": "Recording started" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/replay/{id}/stop": {
            "post": {
                "tags": ["Modules"],
                "summary": "Stop recording",
                "description": "Stop an active flow recording session.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Recording stopped" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/healer/problems": {
            "get": {
                "tags": ["Modules"],
                "summary": "List healer problems",
                "description": "List detected problems that the auto-healer can fix.",
                "responses": {
                    "200": { "description": "List of problems" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/modules/healer/{id}/fix": {
            "post": {
                "tags": ["Modules"],
                "summary": "Apply healer fix",
                "description": "Apply an automated fix for a detected problem. Requires admin role.",
                "parameters": [
                    { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                ],
                "responses": {
                    "200": { "description": "Fix applied" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "404": { "$ref": "#/components/responses/NotFound" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        }
    })
}

fn paths_diagnostics_infra_ebpf_ws() -> serde_json::Value {
    serde_json::json!({
        // ---- Diagnostics ----
        "/diagnostics/run": {
            "post": {
                "tags": ["Diagnostics"],
                "summary": "Run cluster diagnostics",
                "description": "Execute comprehensive diagnostics across Cilium agents, eBPF programs, and network connectivity.",
                "responses": {
                    "200": { "description": "Diagnostics results", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/DiagnosticsResult" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/diagnostics/connectivity": {
            "post": {
                "tags": ["Diagnostics"],
                "summary": "Connectivity test",
                "description": "Run point-to-point connectivity tests between pods, nodes, or services.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "source": { "type": "string", "description": "Source pod or IP" },
                            "destination": { "type": "string", "description": "Destination pod, service, or IP" },
                            "port": { "type": "integer" },
                            "protocol": { "type": "string", "enum": ["TCP", "UDP", "ICMP"] }
                        },
                        "required": ["source", "destination"]
                    } } }
                },
                "responses": {
                    "200": { "description": "Connectivity test result" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },

        // ---- Infrastructure ----
        "/cluster/health": {
            "get": {
                "tags": ["Infrastructure"],
                "summary": "Cluster health overview",
                "description": "Get overall cluster health including node readiness and Cilium agent status.",
                "responses": {
                    "200": { "description": "Cluster health", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ClusterHealth" } } } },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/nodes/drain/status": {
            "get": {
                "tags": ["Infrastructure"],
                "summary": "Node drain status",
                "description": "Check the status of any ongoing node drain operations.",
                "responses": {
                    "200": { "description": "Drain status" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/nodes/drain": {
            "post": {
                "tags": ["Infrastructure"],
                "summary": "Drain a node",
                "description": "Initiate graceful drain of a Kubernetes node. Requires admin role.",
                "requestBody": {
                    "required": true,
                    "content": { "application/json": { "schema": {
                        "type": "object",
                        "properties": {
                            "node_name": { "type": "string" },
                            "grace_period_seconds": { "type": "integer", "default": 300 }
                        },
                        "required": ["node_name"]
                    } } }
                },
                "responses": {
                    "200": { "description": "Drain initiated" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "403": { "$ref": "#/components/responses/Forbidden" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/servicemap": {
            "get": {
                "tags": ["Infrastructure"],
                "summary": "Service dependency map",
                "description": "Get a service-to-service dependency graph derived from observed network flows.",
                "parameters": [
                    { "$ref": "#/components/parameters/namespaceParam" }
                ],
                "responses": {
                    "200": { "description": "Service map with nodes and edges" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/dns/queries": {
            "get": {
                "tags": ["Infrastructure"],
                "summary": "DNS query log",
                "description": "Retrieve DNS queries observed by Cilium's DNS proxy.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" },
                    { "$ref": "#/components/parameters/offsetParam" }
                ],
                "responses": {
                    "200": { "description": "DNS query entries" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },

        // ---- eBPF ----
        "/ebpf/attachments": {
            "get": {
                "tags": ["eBPF"],
                "summary": "Read-only BPF attachment inventory",
                "description": "List loaded BPF programs classified as cilium|netra|other. Never attaches or modifies.",
                "responses": {
                    "200": { "description": "Attachment inventory" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/drift": {
            "get": {
                "tags": ["eBPF"],
                "summary": "BPF brotherhood drift findings",
                "description": "Warn-only findings (inventory unavailable, cilium missing). No datapath mutation.",
                "responses": {
                    "200": { "description": "Drift findings" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/programs": {
            "get": {
                "tags": ["eBPF"],
                "summary": "List eBPF programs",
                "description": "List all loaded eBPF programs via bpftool. Shows program type, attach point, and runtime stats.",
                "responses": {
                    "200": { "description": "List of eBPF programs" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/conntrack": {
            "get": {
                "tags": ["eBPF"],
                "summary": "Conntrack table entries",
                "description": "Dump the Cilium eBPF connection tracking table. Shows active connections with packet/byte counts.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" }
                ],
                "responses": {
                    "200": {
                        "description": "Conntrack entries",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "entries": { "type": "array", "items": { "$ref": "#/components/schemas/ConntrackEntry" } },
                                "total": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/ipcache": {
            "get": {
                "tags": ["eBPF"],
                "summary": "IP cache entries",
                "description": "Dump the Cilium eBPF IP cache mapping CIDRs to security identities.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" }
                ],
                "responses": {
                    "200": {
                        "description": "IP cache entries",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "entries": { "type": "array", "items": { "$ref": "#/components/schemas/IpcacheEntry" } },
                                "total": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/lb": {
            "get": {
                "tags": ["eBPF"],
                "summary": "Load balancer backends",
                "description": "Dump the Cilium eBPF load balancer service/backend map.",
                "parameters": [
                    { "$ref": "#/components/parameters/limitParam" }
                ],
                "responses": {
                    "200": {
                        "description": "LB backend entries",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "entries": { "type": "array", "items": { "$ref": "#/components/schemas/LbBackend" } },
                                "total": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/drops": {
            "get": {
                "tags": ["eBPF"],
                "summary": "Packet drop statistics",
                "description": "Get aggregated packet drop statistics by reason from eBPF metrics maps.",
                "responses": {
                    "200": {
                        "description": "Drop statistics",
                        "content": { "application/json": { "schema": {
                            "type": "object",
                            "properties": {
                                "drop_reasons": { "type": "array", "items": { "$ref": "#/components/schemas/DropStats" } },
                                "total_drops": { "type": "integer" }
                            }
                        } } }
                    },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },
        "/ebpf/summary": {
            "get": {
                "tags": ["eBPF"],
                "summary": "eBPF subsystem summary",
                "description": "Get a high-level summary of all eBPF subsystems: programs loaded, maps in use, conntrack size, etc.",
                "responses": {
                    "200": { "description": "eBPF summary" },
                    "401": { "$ref": "#/components/responses/Unauthorized" },
                    "500": { "$ref": "#/components/responses/InternalError" }
                }
            }
        },

        // ---- WebSocket ----
        "/ws/flows": {
            "get": {
                "tags": ["WebSocket"],
                "summary": "Stream flows (WebSocket)",
                "description": "WebSocket endpoint for streaming network flows in real time. Pass JWT token via `token` query parameter.",
                "security": [],
                "parameters": [
                    { "name": "token", "in": "query", "description": "JWT token for authentication", "schema": { "type": "string" } },
                    { "$ref": "#/components/parameters/namespaceParam" }
                ],
                "responses": {
                    "101": { "description": "WebSocket upgrade successful" },
                    "401": { "$ref": "#/components/responses/Unauthorized" }
                }
            }
        },
        "/ws/flows/live": {
            "get": {
                "tags": ["WebSocket"],
                "summary": "Live flow stream (WebSocket)",
                "description": "WebSocket endpoint for real-time live flow streaming with lower latency than /ws/flows.",
                "security": [],
                "parameters": [
                    { "name": "token", "in": "query", "description": "JWT token for authentication", "schema": { "type": "string" } }
                ],
                "responses": {
                    "101": { "description": "WebSocket upgrade successful" },
                    "401": { "$ref": "#/components/responses/Unauthorized" }
                }
            }
        }
    })
}
