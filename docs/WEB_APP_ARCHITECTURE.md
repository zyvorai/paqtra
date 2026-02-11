# Cilium Vision Web Application Architecture

## Overview

A cloud-native web application for Cilium Vision that provides a modern, real-time dashboard for network observability, policy management, and intelligent automation.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │                    Ingress Controller                   │    │
│  │              (TLS Termination, Routing)                 │    │
│  └────────────────────────────────────────────────────────┘    │
│                           │                                      │
│              ┌────────────┴────────────┐                        │
│              │                         │                        │
│  ┌───────────▼──────────┐  ┌──────────▼──────────┐            │
│  │   Frontend Service   │  │   Backend Service    │            │
│  │   (React SPA)        │  │   (Rust/Axum API)    │            │
│  │   Port: 80           │  │   Port: 8080         │            │
│  │                      │  │                      │            │
│  │  • Dashboard         │  │  • REST API          │            │
│  │  • Flow Viz          │  │  • WebSocket         │            │
│  │  • Policy Management │  │  • GraphQL           │            │
│  │  • Real-time Updates │  │  • gRPC Proxy        │            │
│  └──────────────────────┘  └──────────────────────┘            │
│                                     │                            │
│                      ┌──────────────┴──────────────┐            │
│                      │                             │            │
│          ┌───────────▼──────────┐     ┌───────────▼─────────┐  │
│          │  Cilium Vision Core  │     │   Redis Cache       │  │
│          │  (Intelligence)      │     │   (Sessions/State)  │  │
│          │                      │     └─────────────────────┘  │
│          │  • Anomaly Detection │                              │
│          │  • AutoPolicy        │                              │
│          │  • Security Module   │                              │
│          │  • eBPF Advanced     │                              │
│          └──────────┬───────────┘                              │
│                     │                                           │
│          ┌──────────▼───────────┐                              │
│          │   Hubble Relay       │                              │
│          │   (Flow Observer)    │                              │
│          └──────────────────────┘                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Technology Stack

### Backend (Rust)
- **Framework**: Axum (high-performance async web framework)
- **WebSocket**: tokio-tungstenite
- **GraphQL**: async-graphql
- **Serialization**: serde_json
- **Authentication**: JWT (jsonwebtoken)
- **Cache**: redis-rs
- **Database**: PostgreSQL (sqlx) - optional for persistence
- **Observability**: tracing, prometheus metrics

### Frontend (React)
- **Framework**: React 18 with TypeScript
- **State Management**: Redux Toolkit + RTK Query
- **UI Library**: Material-UI (MUI) or Ant Design
- **Visualization**:
  - D3.js for network topology
  - Recharts for metrics
  - Cytoscape.js for policy graphs
- **Real-time**: Socket.IO or native WebSocket
- **Build Tool**: Vite
- **Testing**: Vitest + React Testing Library

### Infrastructure
- **Container Runtime**: Docker
- **Orchestration**: Kubernetes
- **Ingress**: NGINX Ingress Controller
- **TLS**: cert-manager for automatic certificates
- **Monitoring**: Prometheus + Grafana
- **Logging**: Fluent Bit → Elasticsearch → Kibana

## Component Architecture

### 1. Backend API Server

#### Core Modules
```rust
web-api/
├── src/
│   ├── main.rs                 # Entry point
│   ├── routes/
│   │   ├── mod.rs              # Route definitions
│   │   ├── flows.rs            # Flow monitoring APIs
│   │   ├── policies.rs         # Policy management APIs
│   │   ├── anomalies.rs        # Anomaly detection APIs
│   │   ├── compliance.rs       # Compliance APIs
│   │   └── websocket.rs        # WebSocket handler
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── flow_handler.rs     # Flow business logic
│   │   ├── policy_handler.rs   # Policy business logic
│   │   └── module_handler.rs   # Intelligence modules
│   ├── models/
│   │   ├── mod.rs
│   │   ├── flow.rs             # Flow data models
│   │   ├── policy.rs           # Policy data models
│   │   └── response.rs         # API response models
│   ├── services/
│   │   ├── mod.rs
│   │   ├── hubble_service.rs   # Hubble integration
│   │   ├── k8s_service.rs      # Kubernetes client
│   │   └── cache_service.rs    # Redis cache
│   ├── middleware/
│   │   ├── auth.rs             # JWT authentication
│   │   ├── cors.rs             # CORS configuration
│   │   └── metrics.rs          # Prometheus metrics
│   └── graphql/
│       ├── schema.rs           # GraphQL schema
│       └── resolvers.rs        # Query/Mutation resolvers
```

#### API Endpoints

**Flow Monitoring**
- `GET /api/v1/flows` - List flows with filters
- `GET /api/v1/flows/:id` - Get flow details
- `GET /api/v1/flows/stats` - Flow statistics
- `WS /api/v1/flows/stream` - Real-time flow stream

**Policy Management**
- `GET /api/v1/policies` - List policies
- `POST /api/v1/policies` - Create policy
- `PUT /api/v1/policies/:id` - Update policy
- `DELETE /api/v1/policies/:id` - Delete policy
- `POST /api/v1/policies/simulate` - Dry-run simulation

**Anomaly Detection**
- `GET /api/v1/anomalies` - List detected anomalies
- `GET /api/v1/anomalies/:id` - Anomaly details
- `POST /api/v1/anomalies/:id/remediate` - Apply remediation

**Security & Compliance**
- `GET /api/v1/compliance/frameworks` - List frameworks
- `POST /api/v1/compliance/audit` - Run compliance audit
- `GET /api/v1/security/posture` - Security posture score
- `POST /api/v1/security/zerotrust` - Generate zero-trust policies

**Intelligence Modules**
- `POST /api/v1/modules/autopolicy/generate` - Generate policies
- `GET /api/v1/modules/chaos/experiments` - List experiments
- `POST /api/v1/modules/chaos/run` - Run chaos experiment
- `GET /api/v1/modules/canary/:id` - Canary deployment status

**GraphQL**
- `POST /graphql` - GraphQL queries and mutations
- `WS /graphql/subscriptions` - GraphQL subscriptions

### 2. Frontend Application

#### Page Structure
```
web-ui/
├── src/
│   ├── App.tsx                 # Main app component
│   ├── pages/
│   │   ├── Dashboard.tsx       # Overview dashboard
│   │   ├── Flows.tsx           # Flow monitoring
│   │   ├── Topology.tsx        # Network topology
│   │   ├── Policies.tsx        # Policy management
│   │   ├── Anomalies.tsx       # Anomaly detection
│   │   ├── Compliance.tsx      # Compliance dashboard
│   │   ├── Chaos.tsx           # Chaos engineering
│   │   └── Settings.tsx        # Configuration
│   ├── components/
│   │   ├── FlowTable.tsx       # Flow data table
│   │   ├── NetworkGraph.tsx    # Network visualization
│   │   ├── PolicyEditor.tsx    # YAML policy editor
│   │   ├── MetricsChart.tsx    # Metrics visualization
│   │   └── AnomalyCard.tsx     # Anomaly display
│   ├── features/
│   │   ├── flows/              # Flow feature slice
│   │   ├── policies/           # Policy feature slice
│   │   └── anomalies/          # Anomaly feature slice
│   ├── services/
│   │   ├── api.ts              # REST API client
│   │   └── websocket.ts        # WebSocket client
│   └── hooks/
│       ├── useFlows.ts         # Flow data hook
│       └── useWebSocket.ts     # WebSocket hook
```

#### Dashboard Views

**1. Overview Dashboard**
- Real-time metrics (requests/sec, latency, errors)
- Active connections map
- Policy compliance status
- Recent anomalies
- Security posture score

**2. Flow Monitoring**
- Live flow table with filtering
- Flow details modal
- Packet explanation integration
- Time-travel replay controls

**3. Network Topology**
- Interactive service graph (D3.js)
- Pod-to-pod connections
- Policy enforcement visualization
- Namespace boundaries

**4. Policy Management**
- Policy list with search
- Visual policy editor
- Dry-run simulator integration
- Policy validation

**5. Anomaly Detection**
- Anomaly timeline
- Confidence scoring display
- Remediation actions
- Historical analysis

**6. Compliance Dashboard**
- Framework selection (PCI-DSS, SOC2, HIPAA, etc.)
- Control status grid
- Violation details
- Audit reports

## Data Flow

### Real-time Updates Flow
```
Hubble gRPC → Backend Service → WebSocket → Frontend Components
     │              │                             │
     │              ├─ Redis Cache ←──────────────┘
     │              │
     │              ├─ Intelligence Modules
     │              │  (Anomaly Detection, AutoPolicy)
     │              │
     │              └─ Prometheus Metrics
```

### Request Flow
```
User Action → Frontend → REST API → Backend Handler →
   ↓                                        ↓
   ↓                                   K8s/Hubble
   ↓                                        ↓
   ← JSON Response ← API Response ← Result ←
```

## Security Architecture

### Authentication & Authorization
- **JWT-based authentication**
- **RBAC integration with Kubernetes**
- **API key support for CI/CD**
- **Session management with Redis**

### Security Headers
```yaml
Content-Security-Policy: default-src 'self'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
Strict-Transport-Security: max-age=31536000
```

### TLS Configuration
- Automatic certificate management via cert-manager
- TLS 1.3 minimum
- Strong cipher suites

## Scalability

### Horizontal Scaling
- Frontend: Stateless, can scale infinitely
- Backend: Stateless API pods (3+ replicas recommended)
- Redis: Redis Sentinel for HA
- Database: PostgreSQL with read replicas

### Performance Optimizations
- Response caching with Redis (60s TTL)
- GraphQL query batching
- WebSocket connection pooling
- Frontend code splitting
- CDN for static assets

## Monitoring & Observability

### Metrics (Prometheus)
```
# Backend metrics
http_requests_total{method, path, status}
http_request_duration_seconds{method, path}
websocket_connections_active
hubble_flows_processed_total

# Frontend metrics (via backend proxy)
page_load_time_seconds{page}
api_call_duration_seconds{endpoint}
```

### Logging
```json
{
  "timestamp": "2026-02-11T18:00:00Z",
  "level": "INFO",
  "service": "cilium-vision-api",
  "message": "Policy created",
  "namespace": "production",
  "policy_name": "allow-frontend-backend",
  "user": "admin@example.com"
}
```

### Distributed Tracing
- OpenTelemetry integration
- Jaeger for trace visualization
- Trace context propagation across services

## Deployment Strategy

### Rolling Updates
```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 1
    maxUnavailable: 0
```

### Health Checks
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
```

### Resource Requirements
```yaml
# Backend
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "1Gi"
    cpu: "1000m"

# Frontend
resources:
  requests:
    memory: "64Mi"
    cpu: "100m"
  limits:
    memory: "256Mi"
    cpu: "500m"
```

## Development Workflow

### Local Development
```bash
# Backend
cd web-api
cargo run

# Frontend
cd web-ui
npm run dev

# Docker Compose for full stack
docker-compose up
```

### CI/CD Pipeline
```yaml
Stages:
1. Build & Test
   - Cargo build & test (backend)
   - npm build & test (frontend)
2. Security Scan
   - cargo audit
   - npm audit
   - Trivy container scan
3. Build Images
   - Multi-stage Docker builds
   - Push to registry
4. Deploy to Staging
   - Helm chart deployment
   - Integration tests
5. Deploy to Production
   - Manual approval
   - Helm upgrade
   - Smoke tests
```

## Future Enhancements

### Phase 1 (v1.0)
- Core dashboard and flow monitoring
- Policy management UI
- WebSocket real-time updates
- Basic authentication

### Phase 2 (v1.1)
- Anomaly detection UI
- Compliance dashboard
- Advanced visualizations
- Multi-tenancy support

### Phase 3 (v2.0)
- AI-powered insights
- Mobile app (React Native)
- Advanced RBAC
- Plugin system

---

**Architecture Principles:**
1. **Cloud-Native**: Kubernetes-first design
2. **Scalable**: Horizontal scaling for all components
3. **Observable**: Comprehensive metrics, logs, traces
4. **Secure**: Authentication, authorization, encryption
5. **Performant**: <100ms API latency, 60 FPS frontend

Built with ❤️ using Rust, React, and Kubernetes
