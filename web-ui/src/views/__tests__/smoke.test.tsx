/**
 * Smoke render tests for all key views.
 *
 * Each test verifies:
 *   1. The component renders without crashing.
 *   2. A recognizable heading or title is present in the DOM.
 *
 * API calls are mocked globally so no network requests are made.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ---------------------------------------------------------------------------
// Mock the API module — every exported function resolves with empty data
// ---------------------------------------------------------------------------
vi.mock('../../services/api', () => {
  const emptyGet = vi.fn().mockResolvedValue({ data: {} });
  const emptyPost = vi.fn().mockResolvedValue({ data: {} });
  const emptyPut = vi.fn().mockResolvedValue({ data: {} });
  const emptyDel = vi.fn().mockResolvedValue({ data: {} });

  const apiInstance = {
    get: emptyGet,
    post: emptyPost,
    put: emptyPut,
    delete: emptyDel,
    defaults: { headers: { common: {} } },
    interceptors: {
      request: { use: vi.fn(), handlers: [] },
      response: { use: vi.fn(), handlers: [] },
    },
  };

  return {
    default: apiInstance,
    // Flow helpers
    fetchFlows: vi.fn().mockResolvedValue({ data: { flows: [], total: 0 } }),
    fetchFlowStats: vi.fn().mockResolvedValue({ data: { total_flows: 0, forwarded: 0, dropped: 0, requests_per_second: 0, avg_latency_ms: 0 } }),
    // Policies
    fetchPolicies: vi.fn().mockResolvedValue({ data: { policies: [] } }),
    createPolicy: emptyPost,
    deletePolicy: emptyDel,
    updatePolicy: emptyPut,
    simulatePolicy: emptyPost,
    // Nodes
    fetchNodes: vi.fn().mockResolvedValue({ data: { nodes: [] } }),
    // Endpoints
    fetchEndpoints: vi.fn().mockResolvedValue({ data: { endpoints: [] } }),
    // Events
    fetchEvents: vi.fn().mockResolvedValue({ data: { events: [] } }),
    // Anomalies
    fetchAnomalies: vi.fn().mockResolvedValue({ data: { anomalies: [], total: 0 } }),
    remediateAnomaly: emptyPost,
    // Compliance / Security
    fetchFrameworks: vi.fn().mockResolvedValue({ data: { frameworks: [] } }),
    runAudit: emptyPost,
    fetchSecurityPosture: vi.fn().mockResolvedValue({ data: { score: 0, trend: '-' } }),
    fetchSecurityFindings: vi.fn().mockResolvedValue({ data: { findings: [] } }),
    fetchZeroTrustScore: vi.fn().mockResolvedValue({ data: { overall: 0, network_segmentation: 0, identity_verification: 0, encryption: 0, least_privilege: 0, monitoring: 0 } }),
    // Cluster Health
    fetchClusterHealth: vi.fn().mockResolvedValue({ data: { status: 'healthy', score: 100, node_count: 0, pod_count: 0, endpoint_count: 0, components: [] } }),
    // Service Map
    fetchServiceMap: vi.fn().mockResolvedValue({ data: { nodes: [], edges: [] } }),
    // Heatmap
    fetchHeatmapData: vi.fn().mockResolvedValue({ data: { cells: [], namespaces: [] } }),
    // Replay
    fetchRecordings: vi.fn().mockResolvedValue({ data: { recordings: [] } }),
    startRecording: emptyPost,
    stopRecording: emptyPost,
    fetchRecordingFlows: emptyGet,
    // Root Cause
    fetchPacketDrops: vi.fn().mockResolvedValue({ data: { drops: [] } }),
    analyzeDrops: emptyPost,
    // AutoPolicy
    generateAutopolicy: emptyPost,
    // Chaos
    fetchChaosExperiments: vi.fn().mockResolvedValue({ data: { experiments: [] } }),
    runChaosExperiment: emptyPost,
    // Healer
    fetchHealerProblems: vi.fn().mockResolvedValue({ data: { problems: [] } }),
    applyHealerFix: emptyPost,
    // Topology
    fetchServiceDeps: vi.fn().mockResolvedValue({ data: { dependencies: [] } }),
    // Dashboard helpers
    fetchHostInfo: vi.fn().mockResolvedValue({ data: { hostname: '', os: '', kernel: '', arch: '', cpu_model: '', cpu_cores: 0, cpu_usage: 0, memory_total_gb: 0, memory_used_gb: 0, disk_total_gb: 0, disk_used_gb: 0, uptime_seconds: 0, load_average: [], network_interfaces: [] } }),
    fetchClusters: vi.fn().mockResolvedValue({ data: { clusters: [] } }),
    fetchCiliumStatus: vi.fn().mockResolvedValue({ data: { agents: [] } }),
    checkHealth: emptyGet,
    checkReady: emptyGet,
    // Misc
    fetchCanaryStatus: emptyGet,
    fetchEbpfPrograms: vi.fn().mockResolvedValue({ data: { programs: [] } }),
    fetchEbpfMaps: vi.fn().mockResolvedValue({ data: { maps: [] } }),
    fetchPrometheusMetrics: emptyGet,
    fetchMetricsSummary: vi.fn().mockResolvedValue({ data: {} }),
  };
});

// ---------------------------------------------------------------------------
// Mock recharts — it relies on DOM measurements that jsdom does not support
// ---------------------------------------------------------------------------
vi.mock('recharts', () => {
  const FakeChart = ({ children }: { children?: React.ReactNode }) => <div data-testid="recharts-mock">{children}</div>;
  return {
    ResponsiveContainer: FakeChart,
    LineChart: FakeChart,
    AreaChart: FakeChart,
    BarChart: FakeChart,
    PieChart: FakeChart,
    Pie: FakeChart,
    Cell: () => null,
    Line: () => null,
    Area: () => null,
    Bar: () => null,
    XAxis: () => null,
    YAxis: () => null,
    CartesianGrid: () => null,
    Tooltip: () => null,
    Legend: () => null,
  };
});

// ---------------------------------------------------------------------------
// Stub WebSocket for Dashboard (jsdom has no native WebSocket)
// ---------------------------------------------------------------------------
class MockWebSocket {
  static OPEN = 1;
  readyState = 3; // CLOSED — prevents actual connection attempts
  onopen: (() => void) | null = null;
  onmessage: ((e: unknown) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  close = vi.fn();
  send = vi.fn();
  constructor() {
    // Simulate immediate close so no timers linger
    setTimeout(() => this.onclose?.(), 0);
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
import React from 'react';

function renderView(Component: React.ComponentType) {
  return render(
    <MemoryRouter>
      <Component />
    </MemoryRouter>,
  );
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------
let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  originalWebSocket = globalThis.WebSocket;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (globalThis as any).WebSocket = MockWebSocket;
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  vi.useRealTimers();
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

// ============================= TESTS =======================================

describe('Flows view', () => {
  it('renders the heading', async () => {
    const Flows = (await import('../Flows')).default;
    renderView(Flows);
    expect(screen.getByText('Flow Monitoring')).toBeInTheDocument();
  });
});

describe('Policies view', () => {
  it('renders the heading', async () => {
    const Policies = (await import('../Policies')).default;
    renderView(Policies);
    expect(screen.getByText('Policy Management')).toBeInTheDocument();
  });
});

describe('Nodes view', () => {
  it('renders the heading', async () => {
    const Nodes = (await import('../Nodes')).default;
    renderView(Nodes);
    // "Nodes" appears in both the h1 heading and the stat card label;
    // use a role selector to target the heading specifically.
    expect(screen.getByRole('heading', { name: 'Nodes' })).toBeInTheDocument();
  });
});

describe('Endpoints view', () => {
  it('renders the heading', async () => {
    const Endpoints = (await import('../Endpoints')).default;
    renderView(Endpoints);
    expect(screen.getByText('Cilium Endpoints')).toBeInTheDocument();
  });
});

describe('Events view', () => {
  it('renders the heading', async () => {
    const Events = (await import('../Events')).default;
    renderView(Events);
    expect(screen.getByText('Events')).toBeInTheDocument();
  });
});

describe('Anomalies view', () => {
  it('renders the heading', async () => {
    const Anomalies = (await import('../Anomalies')).default;
    renderView(Anomalies);
    expect(screen.getByText('Anomaly Detection')).toBeInTheDocument();
  });
});

describe('Compliance view', () => {
  it('renders the heading', async () => {
    const Compliance = (await import('../Compliance')).default;
    renderView(Compliance);
    expect(screen.getByText('Security & Compliance')).toBeInTheDocument();
  });
});

describe('ClusterHealth view', () => {
  it('renders the heading', async () => {
    const ClusterHealth = (await import('../ClusterHealth')).default;
    renderView(ClusterHealth);
    expect(screen.getByText('Cluster Health')).toBeInTheDocument();
  });
});

describe('ServiceMap view', () => {
  it('renders the heading', async () => {
    const ServiceMap = (await import('../ServiceMap')).default;
    renderView(ServiceMap);
    expect(screen.getByText('Service Map')).toBeInTheDocument();
  });
});

describe('Heatmap view', () => {
  it('renders the heading', async () => {
    const Heatmap = (await import('../Heatmap')).default;
    renderView(Heatmap);
    expect(screen.getByText('Traffic Heatmap')).toBeInTheDocument();
  });
});

describe('Replay view', () => {
  it('renders the heading', async () => {
    const Replay = (await import('../Replay')).default;
    renderView(Replay);
    expect(screen.getByText('Flow Replay')).toBeInTheDocument();
  });
});

describe('RootCause view', () => {
  it('renders the heading', async () => {
    const RootCause = (await import('../RootCause')).default;
    renderView(RootCause);
    expect(screen.getByText('Root Cause Analysis')).toBeInTheDocument();
  });
});

describe('SecurityDash view', () => {
  it('renders the heading', async () => {
    const SecurityDash = (await import('../SecurityDash')).default;
    renderView(SecurityDash);
    expect(screen.getByText('Security Dashboard')).toBeInTheDocument();
  });
});

describe('Dashboard view', () => {
  it('renders the heading', async () => {
    const Dashboard = (await import('../Dashboard')).default;
    renderView(Dashboard);
    expect(screen.getByText('Cilium Vision')).toBeInTheDocument();
  });
});

describe('AutoPolicy view', () => {
  it('renders the heading', async () => {
    const AutoPolicy = (await import('../AutoPolicy')).default;
    renderView(AutoPolicy);
    expect(screen.getByText('AutoPolicy Engine')).toBeInTheDocument();
  });
});

describe('Chaos view', () => {
  it('renders the heading', async () => {
    const Chaos = (await import('../Chaos')).default;
    renderView(Chaos);
    expect(screen.getByText('Chaos Engineering')).toBeInTheDocument();
  });
});

describe('Healer view', () => {
  it('renders the heading', async () => {
    const Healer = (await import('../Healer')).default;
    renderView(Healer);
    expect(screen.getByText('Network Healer')).toBeInTheDocument();
  });
});

describe('Topology view', () => {
  it('renders the heading', async () => {
    const Topology = (await import('../Topology')).default;
    renderView(Topology);
    expect(screen.getByText('Network Topology')).toBeInTheDocument();
  });
});
