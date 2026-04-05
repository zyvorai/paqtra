# Cilium Vision Web Application Architecture

## Overview

Cloud-native web application providing a real-time dashboard for Cilium network observability, policy management, and intelligent automation. Built with React 19, TypeScript, and Tailwind CSS.

## Architecture Diagram

```
+-------------------------------------------------------------+
|                    Kubernetes Cluster                         |
+-------------------------------------------------------------+
|                                                               |
|  +---------------------------+  +--------------------------+ |
|  |   Frontend (React SPA)    |  |   Backend (Rust/Actix)   | |
|  |   Port: 3000 (dev)        |  |   Port: 9191             | |
|  |                            |  |                          | |
|  |  25 shared components      |  |  REST API (/api/v1/*)   | |
|  |  58 lazy-loaded views      |  |  WebSocket (/ws/*)      | |
|  |  Zustand state (3 stores)  |  |  JWT authentication     | |
|  |  WebSocket real-time       |  |  Hubble gRPC proxy      | |
|  +---------------------------+  +--------------------------+ |
|                                          |                    |
|                           +--------------+---------------+   |
|                           |                              |   |
|               +-----------v-----------+  +---------------v-+ |
|               |  Cilium Vision Core   |  |  Hubble Relay   | |
|               |  (Intelligence)       |  |  (Flow Data)    | |
|               |                       |  +-----------------+ |
|               |  AutoPolicy Engine    |                      |
|               |  Anomaly Detection    |                      |
|               |  Chaos Engineering    |                      |
|               |  eBPF Profiler        |                      |
|               +-----------------------+                      |
+-------------------------------------------------------------+
```

## Technology Stack

### Backend (Rust)
- **Framework**: Actix-Web
- **WebSocket**: actix-ws
- **Serialization**: serde_json
- **Authentication**: JWT (Bearer tokens)
- **Observability**: tracing, prometheus metrics

### Frontend (React)
- **Framework**: React 19 with TypeScript 5.6
- **State Management**: Zustand 5 (auth, theme, preferences)
- **Data Fetching**: TanStack React Query 5.56 + Axios
- **Routing**: React Router 6.28 with React.lazy code splitting
- **Visualization**: Recharts 2.15, D3 (d3-force, d3-selection, d3-drag)
- **Code Editor**: Monaco Editor (YAML policy editing)
- **Icons**: Lucide React
- **Styling**: Tailwind CSS 3.4 (dark-first, class-based toggle)
- **Build Tool**: Vite 6 (manual chunk splitting, sourcemaps)
- **Testing**: Vitest 2 + React Testing Library (68 tests)

## Frontend Architecture

### Component Layer (25 components)

```
components/
  Layout:       MainLayout, LoginPage
  Data Display: StatCard, ChartContainer, SortableTable, ResponsiveTable,
                AlertsList, Badge, ProgressBar, ScoreGauge, Sparkline
  Feedback:     Toast, LoadingSpinner, Skeleton, ErrorBoundary, ErrorRetry,
                EmptyState, LiveBadge
  Navigation:   GlobalSearch, Breadcrumbs, QuickLinks
  Form:         ToggleSwitch, Accordion, ExportButton, NotificationManager
```

### State Management

| Store | Purpose | Persistence |
|-------|---------|-------------|
| authStore | Token, username, session check | localStorage + sessionStorage |
| themeStore | Dark/light toggle | localStorage |
| preferencesStore | Page size, refresh interval, etc. | localStorage |

### Hooks

| Hook | Purpose |
|------|---------|
| useWebSocket | Auto-reconnect WS with JSON parsing |
| useMetricsHistory | Sliding-window buffer for time-series data |
| useKeyboardShortcuts | Global shortcuts with g-prefix navigation |
| usePageTitle | Dynamic document.title |

### View Layer (58 views)

All views are lazy-loaded via `React.lazy()` + `Suspense` for optimal code splitting.

Each view follows a consistent pattern:
1. Gradient icon box page header
2. Error/success alert banners
3. Stat cards with card-glow hover effects
4. Data tables with sortable headers, section headers, count badges
5. Charts with gradient icon headers and dark tooltip styling

### API Layer

Single Axios instance (`services/api.ts`) with:
- Auto-injected Bearer token from sessionStorage
- 401 response interceptor (clears auth state)
- 49 typed interfaces for all API responses
- 70+ typed API functions

## Design System

Dark-first theme matching HyperSDK patterns:

### Color Palette
- **Page**: `bg-slate-950` / `#0f172a`
- **Cards**: `bg-slate-800/50` with `border-slate-700/50`
- **Inputs**: `bg-slate-900/50` with `border-slate-700/50`
- **Primary text**: `text-white`
- **Secondary text**: `text-slate-400`
- **Muted text**: `text-slate-500`

### Stat Card System
Six gradient variants: `stat-card-{blue,green,red,purple,orange,cyan}`
Each with matching `card-glow-{color}` hover effect and `hover:scale-[1.02]`

### Component Patterns
- **Gradient icon box**: `w-10 h-10 rounded-lg bg-gradient-to-br from-{color}-500 to-{color}-700`
- **Section accent bar**: `w-1 h-5 bg-gradient-to-b from-{color}-400 to-{color}-500 rounded-full`
- **Terminal header**: Traffic light dots (red/yellow/green circles)
- **Table headers**: `uppercase tracking-wider font-semibold text-slate-400`
- **Buttons**: Gradient primary, border secondary, gradient-to-r reset

### Animations
- `animate-fade-in`: translateY(8px) + opacity
- `animate-scale-in`: scale(0.95) + opacity
- `animate-slide-in`: translateX(100%) + opacity
- `animate-pulse-dot`: opacity pulse for live indicators
- `skeleton`: shimmer gradient for loading states

### Light Theme
Full light theme support via `.light-theme` class + `html:not(.dark)` CSS variables.
Toggle persisted in localStorage.

## API Endpoints

### Core
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | Health check |
| POST | `/auth/login` | Authentication |
| WS | `/api/v1/ws/metrics` | Real-time metrics stream |

### Observability
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/flows` | Paginated flows with filters |
| GET | `/api/v1/flows/stats` | Flow statistics |
| GET | `/api/v1/events` | Kubernetes events |
| GET | `/api/v1/nodes` | Node status and resources |
| GET | `/api/v1/endpoints` | Cilium endpoints |
| GET | `/api/v1/heatmap` | Traffic heatmap data |
| GET | `/api/v1/service-deps` | Service dependency graph |
| GET | `/api/v1/dns/queries` | DNS query monitoring |
| GET | `/api/v1/latency` | Latency percentile analysis |

### Security & Policy
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/policies` | List network policies |
| POST | `/api/v1/policies` | Create policy |
| POST | `/api/v1/policies/validate` | Validate YAML |
| GET | `/api/v1/anomalies` | Detected anomalies |
| GET | `/api/v1/security/findings` | Security findings |
| GET | `/api/v1/security/zero-trust` | Zero-trust score |
| GET | `/api/v1/compliance/frameworks` | Compliance frameworks |

### Intelligence
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/modules/autopolicy/generate` | ML policy generation |
| GET | `/api/v1/healer/problems` | Detected problems |
| GET | `/api/v1/packet-drops` | Drop analysis |
| POST | `/api/v1/diagnostics/run` | Run health checks |
| POST | `/api/v1/troubleshoot` | Connectivity test |

### Operations
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/modules/chaos/experiments` | Chaos experiments |
| POST | `/api/v1/modules/chaos/run` | Run experiment |
| GET | `/api/v1/recordings` | Flow recordings |
| GET | `/api/v1/capture/sessions` | Packet captures |
| GET | `/api/v1/clusters` | Multi-cluster status |

## Development

```bash
# Frontend
cd web-ui
npm install
npm run dev        # Vite dev server (port 3000, proxies to :9191)
npm run build      # Production build (tsc + vite)
npm run test       # 68 tests (vitest)
npm run lint       # ESLint (0 errors)

# Backend
cd web-api
cargo run          # API server on port 9191

# Full stack
docker-compose up
```

## Build & Performance

| Metric | Value |
|--------|-------|
| Production build | ~6s |
| Bundle size (gzip) | ~220 KB total |
| Code splitting | 4 vendor chunks + per-view lazy loading |
| TypeScript | Strict mode, 0 errors |
| ESLint | 0 errors, 0 warnings |
| Test suite | 68 tests, 9 files |
| Views | 58 lazy-loaded pages |
| Components | 25 shared UI components |
