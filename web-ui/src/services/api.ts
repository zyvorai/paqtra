import axios, { AxiosError, InternalAxiosRequestConfig } from 'axios';

/** Settings stored in localStorage by the Settings page */
interface AppSettings {
  apiBaseUrl: string;
  hubbleAddress: string;
  refreshInterval: number;
  darkMode: boolean;
}

const DEFAULT_SETTINGS: AppSettings = {
  apiBaseUrl: '/api/v1',
  hubbleAddress: 'localhost:4245',
  refreshInterval: 5,
  darkMode: true,
};

function isValidApiBaseUrl(url: string): boolean {
  // Allow relative paths starting with /
  if (url.startsWith('/')) return true;
  // Only allow same-origin absolute URLs to prevent SSRF
  try {
    const parsed = new URL(url);
    return parsed.origin === window.location.origin;
  } catch {
    return false;
  }
}

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem('cilium-vision-settings');
    if (raw) {
      const merged = { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
      // Validate API base URL to prevent SSRF
      if (!isValidApiBaseUrl(merged.apiBaseUrl)) {
        console.warn('[api] Invalid apiBaseUrl in settings, using default');
        merged.apiBaseUrl = DEFAULT_SETTINGS.apiBaseUrl;
      }
      return merged;
    }
  } catch {
    // ignore
  }
  return DEFAULT_SETTINGS;
}

/**
 * Shared Axios instance with:
 * - Base URL from localStorage settings (or /api/v1)
 * - Authorization header injection
 * - Response error logging
 */
const api = axios.create({
  baseURL: loadSettings().apiBaseUrl,
  timeout: 15_000,
  headers: { 'Content-Type': 'application/json' },
});

// --- Request interceptor ---------------------------------------------------
api.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  // Attach JWT token if available
  const token = localStorage.getItem('cilium-vision-token');
  if (token && config.headers) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// --- Response interceptor ---------------------------------------------------
api.interceptors.response.use(
  (response) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      // Token expired or invalid – clear and let the UI handle it
      localStorage.removeItem('cilium-vision-token');
      // In production, send to observability service (e.g., Sentry, Datadog)
      console.warn('[api] Unauthorized – token cleared');
    }
    return Promise.reject(error);
  },
);

export default api;

// ─── Typed API helpers ──────────────────────────────────────────────────────

// Flows
export interface FlowEndpoint {
  namespace: string;
  pod: string;
  ip: string;
  [key: string]: unknown;
}

export interface Flow {
  id: string;
  timestamp: string;
  source: FlowEndpoint;
  destination: FlowEndpoint;
  verdict: string;
  protocol: string;
  port: number;
  [key: string]: unknown;
}

export interface FlowStats {
  total_flows: number;
  forwarded: number;
  dropped: number;
  requests_per_second: number;
  avg_latency_ms: number;
  [key: string]: unknown;
}

export const fetchFlows = (params?: {
  namespace?: string;
  verdict?: string;
  limit?: number;
  offset?: number;
}) => api.get<{ flows: Flow[]; total: number }>('/flows', { params });

export const fetchFlowStats = () => api.get<FlowStats>('/flows/stats');

// Policies
export interface Policy {
  id: string;
  name: string;
  namespace: string;
  created_at: string;
  status: string;
  [key: string]: unknown;
}

export const fetchPolicies = () =>
  api.get<{ policies: Policy[] }>('/policies');

export const createPolicy = (body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.post('/policies', body);

export const updatePolicy = (id: string, body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.put(`/policies/${id}`, body);

export const deletePolicy = (id: string) => api.delete(`/policies/${id}`);

export const simulatePolicy = (body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.post('/policies/simulate', body);

// Anomalies
export interface Anomaly {
  id: string;
  detected_at: string;
  severity: string;
  anomaly_type: string;
  description: string;
  source_namespace: string;
  source_pod: string | null;
  destination_namespace: string | null;
  destination_pod: string | null;
  status: string;
  remediation: string | null;
  [key: string]: unknown;
}

export const fetchAnomalies = () =>
  api.get<{ anomalies: Anomaly[]; total: number }>('/anomalies');

export const remediateAnomaly = (id: string) =>
  api.post(`/anomalies/${id}/remediate`);

// Compliance
export const fetchFrameworks = () =>
  api.get<{ frameworks: string[] }>('/compliance/frameworks');

export const runAudit = (framework: string) =>
  api.post('/compliance/audit', { framework });

export const fetchSecurityPosture = () =>
  api.get<{ score: number; trend: string }>('/security/posture');

// Modules
export const generateAutopolicy = (body: unknown) =>
  api.post('/modules/autopolicy/generate', body);

export const fetchChaosExperiments = () =>
  api.get('/modules/chaos/experiments');

export const runChaosExperiment = (body: unknown) =>
  api.post('/modules/chaos/run', body);

export const fetchCanaryStatus = (id: string) =>
  api.get(`/modules/canary/${id}`);

// Health
export const checkHealth = () => api.get('/health');
export const checkReady = () => api.get('/ready');

// ─── Extended Interfaces ───────────────────────────────────────────────────────

export interface K8sEvent {
  id: string;
  type: string;
  reason: string;
  object: string;
  message: string;
  namespace: string;
  count: number;
  first_timestamp: string;
  last_timestamp: string;

  last_seen?: string;
  [key: string]: unknown;
}

export interface CiliumEndpoint {
  id: string;
  name: string;
  namespace: string;
  status: string;
  identity: string;
  ipv4: string;
  ipv6: string;
  labels: string[];
  policy_enforcement: string;
  [key: string]: unknown;
}

export interface K8sNode {
  name: string;
  status: string;
  roles: string[];
  version: string;
  os: string;
  kernel: string;
  cpu_capacity: number;
  cpu_usage: number;
  memory_capacity_gb: number;
  memory_usage_gb: number;
  pods: number;
  age: string;

  pods_capacity?: number;
  pods_count?: number;
  [key: string]: unknown;
}

export interface ReplayRecording {
  id: string;
  name: string;
  namespace: string;
  start_time: string;
  end_time: string;
  flow_count: number;
  status: string;
  size: number;

  size_bytes?: number;
  [key: string]: unknown;
}

export interface HealerProblem {
  id: string;
  type: string;
  severity: string;
  description: string;
  affected_pods: string[];
  namespace: string;
  detected_at: string;
  status: string;
  proposed_fix: string;
  [key: string]: unknown;
}

export interface PacketDrop {
  id: string;
  timestamp: string;
  source: string;
  destination: string;
  protocol: string;
  drop_reason: string;
  root_cause: string;
  remediation: string;
  count: number;
  [key: string]: unknown;
}

export interface ClusterInfo {
  latency_ms?: number;
  name: string;
  status: string;
  endpoint: string;
  region: string;
  nodes: number;
  pods: number;
  latency: number;
  last_sync: string;
  cilium_version: string;
  [key: string]: unknown;
}

export interface HeatmapCell {
  source_namespace: string;
  dest_namespace: string;
  flow_count: number;
  dropped_count: number;
  avg_latency: number;

  destination_namespace?: string;
  avg_latency_ms?: number;
  [key: string]: unknown;
}

export interface ServiceDep {
  source: string;
  destination: string;
  protocol: string;
  port: number;
  request_rate: number;
  error_rate: number;
  latency_p50: number;
  latency_p99: number;
  [key: string]: unknown;
}

export interface SecurityFinding {
  id: string;
  severity: string;
  category: string;
  title: string;
  description: string;
  resource: string;
  namespace: string;
  status: string;
  remediation: string;
  [key: string]: unknown;
}

export interface ZeroTrustScore {
  overall: number;
  network_segmentation: number;
  identity_verification: number;
  encryption: number;
  least_privilege: number;
  monitoring: number;
  [key: string]: unknown;
}

export interface EbpfProgram {
  id: string;
  name: string;
  type: string;
  attached_to: string;
  run_count: number;
  run_time_ns: number;
  map_ids: number[];

  attach_point?: string;
  avg_run_time_ns?: number;
  map_count?: number;
  [key: string]: unknown;
}

export interface EbpfMapInfo {
  id: string;
  name: string;
  type: string;
  key_size: number;
  value_size: number;
  max_entries: number;
  current_entries: number;
  flags: string;
  [key: string]: unknown;
}

export interface MetricsSummary {
  request_count: number;
  error_count: number;
  cache_hits: number;
  cache_misses: number;
  uptime_seconds: number;

  total_errors?: number;
  total_queries?: number;
  total_requests?: number;
  [key: string]: unknown;
}

export interface HostInfo {
  hostname: string;
  os: string;
  kernel: string;
  arch: string;
  cpu_model: string;
  cpu_cores: number;
  cpu_usage: number;
  memory_total_gb: number;
  memory_used_gb: number;
  disk_total_gb: number;
  disk_used_gb: number;
  uptime_seconds: number;
  load_average: number[];
  network_interfaces: { name: string; ip: string; mac: string; speed: string; status: string }[];
}

export interface PolicyTemplate {
  id: string;
  name: string;
  category: string;
  description: string;
  yaml: string;
  tags: string[];
  [key: string]: unknown;
}

export interface DiagnosticTest {
  name: string;
  status: string;
  message: string;
  duration_ms: number;
  [key: string]: unknown;
}

export interface AuditEntry {
  id: string;
  timestamp: string;
  action: string;
  actor: string;
  resource: string;
  namespace: string;
  details: string;
  outcome: string;
  [key: string]: unknown;
}

export interface AlertRule {
  id: string;
  name: string;
  severity: string;
  condition: string;
  enabled: boolean;
  channels: string[];
  trigger_count: number;
  last_triggered: string;
  [key: string]: unknown;
}

export interface AlertEvent {
  id: string;
  rule_id: string;
  timestamp: string;
  status: string;
  severity: string;
  message: string;
  namespace: string;
  pod: string;

  rule_name?: string;
  fired_at?: string;
  resolved_at?: string;
  [key: string]: unknown;
}

export interface ServiceNode {
  id: string;
  name: string;
  namespace: string;
  type: string;

  error_rate?: number;
  pods?: number;
  request_rate?: number;
  status?: string;
  [key: string]: unknown;
}

export interface ServiceEdge {
  source: string;
  target: string;
  protocol: string;
  port: number;
  request_rate: number;
  error_rate: number;
  [key: string]: unknown;
}

export interface CaptureSession {
  id: string;
  name: string;
  target_pod: string;
  namespace: string;
  interface_name: string;
  filter: string;
  status: string;
  packet_count: number;
  size: number;
  started_at: string;

  size_bytes?: number;
  [key: string]: unknown;
}

export interface DnsQuery {
  id: string;
  timestamp: string;
  source_pod: string;
  namespace: string;
  query_name: string;
  query_type: string;
  response_code: string;
  response_ips: string[];
  latency_ms: number;
  [key: string]: unknown;
}

export interface DnsStats {
  total_queries: number;
  successful: number;
  nxdomain: number;
  servfail: number;
  avg_latency_ms: number;
  [key: string]: unknown;
}

export interface CiliumIdentity {
  id: number;
  labels: string[];
  namespace: string;
  endpoints_count: number;
  policy_count: number;
  created_at: string;
  [key: string]: unknown;
}

export interface MeshPeer {
  latency_ms?: number;
  name: string;
  endpoint: string;
  status: string;
  connected_since: string;
  synced_identities: number;
  synced_endpoints: number;
  synced_services: number;
  latency: number;
  [key: string]: unknown;
}

export interface BgpPeer {
  name: string;
  peer_address: string;
  peer_asn: number;
  local_asn: number;
  state: string;
  uptime: string;
  prefixes_received: number;
  prefixes_advertised: number;
  messages_received: number;
  messages_sent: number;
  [key: string]: unknown;
}

export interface BandwidthEntry {
  pod: string;
  namespace: string;
  egress_rate_mbps: number;
  ingress_rate_mbps: number;
  egress_limit_mbps: number;
  ingress_limit_mbps: number;
  total_bytes_tx: number;
  total_bytes_rx: number;
  [key: string]: unknown;
}

export interface CostBreakdown {
  namespace: string;
  cpu_cost: number;
  memory_cost: number;
  network_cost: number;
  storage_cost: number;
  total_cost: number;

  trend?: string;
  [key: string]: unknown;
}

export interface CostSummary {
  total_monthly: number;
  total_monthly_cost: number;
  total_daily: number;

  cost_trend?: string;
  savings_potential?: number;
  trend?: string;
  [key: string]: unknown;
}

export interface ForecastResult {
  metric: string;
  unit: string;
  recommendation?: string;
  points: { timestamp: string; actual?: number; predicted: number; upper_bound: number; lower_bound: number; [key: string]: unknown }[];
}

export interface EncryptionStatus {
  enabled: boolean;
  type: string;
  nodes_encrypted: number;
  nodes_total: number;
  interfaces: { name: string; peer: string; endpoint: string; latest_handshake: string; tx_bytes: number; rx_bytes: number; interface?: string; public_key?: string; node?: string; stats?: Record<string, unknown>; [key: string]: unknown }[];
  key_rotation?: string;
  key_rotation_at: string;
  [key: string]: unknown;
}

export interface LBService {
  frontend?: string;
  name: string;
  namespace: string;
  type: string;
  frontend_ip: string;
  frontend_port: number;
  protocol: string;
  backends: { ip: string; port: number; weight: number; state: string; address?: string }[];
  session_affinity: string;
  algorithm: string;
}

export interface IngressRoute {
  name: string;
  namespace: string;
  type: string;
  hosts: string[];
  paths: string[];
  tls: string;
  status: string;
  class_name: string;
  [key: string]: unknown;
}

export interface IPAMPool {
  name: string;
  cidr: string;
  total: number;
  allocated: number;
  available: number;

  usage_pct?: number;
  utilization?: number;
  [key: string]: unknown;
}

export interface IPAllocation {
  ip: string;
  pod: string;
  namespace: string;
  node: string;
  pool: string;
  [key: string]: unknown;
}

export interface LatencyBreakdown {
  service: string;
  p50: number;
  p90: number;
  p95: number;
  p99: number;
  max: number;
  histogram: { range: string; count: number; bucket?: string; [key: string]: unknown }[];
  p50_ms?: number;
  p90_ms?: number;
  p95_ms?: number;
  p99_ms?: number;
  max_ms?: number;
  [key: string]: unknown;
}

export interface MirrorRule {
  id: string;
  name: string;
  source_selector: string;
  destination: string;
  mirror_to: string;
  status: string;
  mirrored_packets: number;
  created_at: string;
  source?: string;
  mirror?: { service: string; namespace: string; port: number };

  stats?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ClusterHealthSummary {
  status: string;
  score: number;
  node_count: number;
  pod_count: number;
  endpoint_count: number;
  components: { name: string; status: string; message: string; uptime?: string; last_check?: string; [key: string]: unknown }[];
  kubernetes?: { nodes?: number; pods?: number; endpoints?: number; version?: string; [key: string]: unknown };
  health_score?: number;
  overall?: string;
  cilium_version?: string;
  kubernetes_version?: string;
  cilium?: { version?: string; [key: string]: unknown };
  [key: string]: unknown;
}

export interface RBACBinding {
  subject: string;
  subject_kind: string;
  role: string;
  role_kind: string;
  namespace: string;
  permissions: string[];
  subjects?: { name?: string; kind?: string; namespace?: string }[];
  [key: string]: unknown;
}

export interface NetInterface {
  name: string;
  node: string;
  type: string;
  mtu: number;
  state: string;
  mac: string;
  ipv4: string;
  rx_bytes: number;
  tx_bytes: number;
  rx_packets: number;
  tx_packets: number;
  rx_errors: number;
  tx_errors: number;

  address?: string;
  addresses?: string[];
  stats?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface TroubleshootResult {
  step: string;
  status: string;
  output: string;
  duration_ms: number;

  name?: string;
  [key: string]: unknown;
}

export interface WireGuardPeer {
  public_key: string;
  endpoint: string;
  allowed_ips: string[];
  latest_handshake: string;
  transfer_rx: number;
  transfer_tx: number;
  persistent_keepalive: number;
  node: string;
  [key: string]: unknown;
}

export interface CiliumAgentStatus {
  node: string;
  status: string;
  version: string;
  uptime: string;
  controllers: number;
  endpoint_count: number;
  policy_revision: number;
  proxy_redirects: number;
  identity_count: number;
  datapath: string;
  masquerading: string;
  encryption: string;
  kpr: string;

  controllers_failing?: number;
  controllers_total?: number;
  kube_proxy_replacement?: string;
  [key: string]: unknown;
}

export interface ExportConfig {
  id: string;
  name: string;
  format: string;
  destination: string;
  filter: string;
  status: string;
  exported_count: number;
  last_export: string;
  [key: string]: unknown;
}

export interface SLOTarget {
  name: string;
  service: string;
  metric: string;
  target: number;
  current: number;
  budget_remaining: number;
  budget_total: number;
  window: string;
  status: string;
  [key: string]: unknown;
}

export interface Incident {
  id: string;
  title: string;
  severity: string;
  status: string;
  started_at: string;
  resolved_at: string;
  duration: string;
  affected_services: string[];
  root_cause: string;
  timeline: { time: string; event: string; actor?: string; [key: string]: unknown }[];
  duration_minutes?: number;
  [key: string]: unknown;
}

export interface ChangeEntry {
  id: string;
  timestamp: string;
  type: string;
  resource: string;
  namespace: string;
  diff_summary: string;
  author: string;
  rollback_available: boolean;
  [key: string]: unknown;
}

export interface NodeDrainStatus {
  node: string;
  status: string;
  pods_evicted: number;
  pods_remaining: number;
  started_at: string;
  cordon: boolean;
  [key: string]: unknown;
}

export interface PodSecurityReport {
  namespace: string;
  enforce_level: string;
  audit_level: string;
  warn_level: string;
  total_pods: number;
  compliant_pods: number;
  violations: { pod: string; policy: string; message: string; violation?: string }[];
}

export interface EgressPolicy {
  name: string;
  namespace: string;
  gateway_node: string;
  egress_ip: string;
  destination_cidrs: string[];
  selectors: string | string[];
  status: string;
  [key: string]: unknown;
}

export interface MeshService {
  timeout_ms?: number;
  name: string;
  namespace: string;
  protocol: string;
  mtls: string;
  retries: number;
  timeout: string;
  circuit_breaker: string;
  traffic_policy: string;
  [key: string]: unknown;
}

export interface KPRStatus {
  enabled: boolean;
  mode: string;
  device: string;
  dsr_mode: string;
  session_affinity: string;
  graceful_termination: string;
  nodeport_range: string;
  services: number;
  backends: number;
  nat_entries: number;
  ct_entries: number;

  node_port_range?: string;
  [key: string]: unknown;
}

// ─── Extended API Functions ────────────────────────────────────────────────────

// Events
export const fetchEvents = () => api.get<{ events: K8sEvent[] }>('/events');

// Endpoints
export const fetchEndpoints = () => api.get<{ endpoints: CiliumEndpoint[] }>('/endpoints');

// Nodes
export const fetchNodes = () => api.get<{ nodes: K8sNode[] }>('/nodes');

// Replay / Recordings
export const fetchRecordings = () => api.get<{ recordings: ReplayRecording[] }>('/modules/replay/recordings');
export const startRecording = (body: unknown) => api.post('/modules/replay/start', body);
export const stopRecording = (id: string) => api.post(`/modules/replay/${id}/stop`);
export const fetchRecordingFlows = (id: string) => api.get(`/modules/replay/${id}/flows`);

// Healer
export const fetchHealerProblems = () => api.get<{ problems: HealerProblem[] }>('/modules/healer/problems');
export const applyHealerFix = (id: string) => api.post(`/modules/healer/${id}/fix`);

// Packet Drops / RootCause
export const fetchPacketDrops = () => api.get<{ drops: PacketDrop[] }>('/modules/rootcause/drops');
export const analyzeDrops = (_body?: unknown) => api.post('/modules/rootcause/analyze');

// Clusters
export const fetchClusters = () => api.get<{ clusters: ClusterInfo[] }>('/modules/multicluster/clusters');
export const syncClusterPolicies = (name: string) => api.post(`/modules/multicluster/${name}/sync`);

// Heatmap
export const fetchHeatmapData = () => api.get<{ cells: HeatmapCell[]; namespaces?: string[] }>('/heatmap');

// Service Dependencies
export const fetchServiceDeps = () => api.get<{ dependencies: ServiceDep[] }>('/dependencies');

// Security
export const fetchSecurityFindings = () => api.get<{ findings: SecurityFinding[] }>('/security/findings');
export const fetchZeroTrustScore = () => api.get<ZeroTrustScore>('/security/zero-trust');

// eBPF
export const fetchEbpfPrograms = () => api.get<{ programs: EbpfProgram[] }>('/modules/ebpf/programs');
export const fetchEbpfMaps = () => api.get<{ maps: EbpfMapInfo[] }>('/modules/ebpf/maps');

// Metrics
export const fetchPrometheusMetrics = () => api.get('/metrics/prometheus');
export const fetchMetricsSummary = () => api.get<MetricsSummary>('/metrics/summary');

// Host
export const fetchHostInfo = () => api.get<HostInfo>('/host/info');

// Policy Templates
export const fetchPolicyTemplates = () => api.get<{ templates: PolicyTemplate[] }>('/policies/templates');
export const applyTemplate = (id: string, _body?: unknown) => api.post(`/policies/templates/${id}/apply`);

// Diagnostics
export const runDiagnostics = () => api.post<{ tests: DiagnosticTest[] }>('/diagnostics/run');
export const fetchConnectivityTest = () => api.post('/diagnostics/connectivity');

// Audit
export const fetchAuditLog = () => api.get<{ entries: AuditEntry[] }>('/audit/log');

// Alerts
export const fetchAlertRules = () => api.get<{ rules: AlertRule[] }>('/alerts/rules');
export const fetchAlertHistory = () => api.get<{ alerts: AlertEvent[]; events?: AlertEvent[] }>('/alerts/history');
export const toggleAlertRule = (id: string) => api.put(`/alerts/rules/${id}`);

// Service Map
export const fetchServiceMap = () => api.get<{ nodes: ServiceNode[]; edges: ServiceEdge[] }>('/servicemap');

// Packet Capture
export const fetchCaptureSessions = () => api.get<{ sessions: CaptureSession[] }>('/modules/capture/sessions');
export const startCapture = (body: unknown) => api.post('/modules/capture/start', body);
export const stopCapture = (id: string) => api.post(`/modules/capture/${id}/stop`);

// DNS
export const fetchDnsQueries = (params?: { namespace?: string; query_name?: string }) =>
  api.get<{ queries: DnsQuery[] }>('/dns/queries', { params });
export const fetchDnsStats = () => api.get<DnsStats>('/dns/stats');

// Identities
export const fetchIdentities = () => api.get<{ identities: CiliumIdentity[] }>('/identities');

// Cluster Mesh
export const fetchMeshPeers = () => api.get<{ peers: MeshPeer[] }>('/clustermesh/peers');
export const connectMeshPeer = (body: unknown) => api.post('/clustermesh/connect', body);

// BGP
export const fetchBgpPeers = () => api.get<{ peers: BgpPeer[] }>('/bgp/peers');

// Bandwidth
export const fetchBandwidthData = () => api.get<{ entries: BandwidthEntry[] }>('/bandwidth');

// Cost
export const fetchCostBreakdown = () => api.get<{ breakdown: CostBreakdown[]; summary: CostSummary }>('/costs/breakdown');

// Forecast
export const fetchForecast = (body: unknown) => api.post<ForecastResult>('/forecast', body);
export const fetchForecastMetrics = () => api.get<{ metrics: string[] }>('/forecast/metrics');

// Encryption
export const fetchEncryptionStatus = () => api.get<EncryptionStatus>('/encryption/status');

// Load Balancing
export const fetchLBServices = () => api.get<{ services: LBService[] }>('/loadbalancer/services');

// Ingress
export const fetchIngressRoutes = () => api.get<{ routes: IngressRoute[] }>('/ingress/routes');

// IPAM
export const fetchIPAMPools = () => api.get<{ pools: IPAMPool[] }>('/ipam/pools');
export const fetchIPAllocations = () => api.get<{ allocations: IPAllocation[] }>('/ipam/allocations');

// Latency
export const fetchLatencyAnalysis = () => api.get<{ services: LatencyBreakdown[] }>('/latency/analysis');

// Mirror
export const fetchMirrorRules = () => api.get<{ rules: MirrorRule[] }>('/modules/mirror/rules');
export const createMirrorRule = (body: unknown) => api.post('/modules/mirror/rules', body);
export const deleteMirrorRule = (id: string) => api.delete(`/modules/mirror/rules/${id}`);

// Cluster Health
export const fetchClusterHealth = () => api.get<ClusterHealthSummary>('/cluster/health');

// RBAC
export const fetchRBACBindings = () => api.get<{ bindings: RBACBinding[] }>('/rbac/bindings');

// Network Interfaces
export const fetchNetInterfaces = () => api.get<{ interfaces: NetInterface[] }>('/network/interfaces');

// Troubleshoot
export const runTroubleshoot = (body: unknown) => api.post<{ results: TroubleshootResult[] }>('/troubleshoot/run', body);

// WireGuard
export const fetchWireGuardPeers = () => api.get<{ peers: WireGuardPeer[] }>('/wireguard/peers');

// Cilium Status
export const fetchCiliumStatus = () => api.get<{ agents: CiliumAgentStatus[] }>('/cilium/status');

// Policy Validation
export const validatePolicy = (yaml: string) => api.post('/policies/validate', { yaml });

// Export
export const fetchExportConfigs = () => api.get<{ configs: ExportConfig[] }>('/flows/exports');
export const createExportConfig = (body: unknown) => api.post('/flows/exports', body);
export const deleteExportConfig = (id: string) => api.delete(`/flows/exports/${id}`);

// SLOs
export const fetchSLOs = () => api.get<{ slos: SLOTarget[] }>('/slo/targets');

// Incidents
export const fetchIncidents = () => api.get<{ incidents: Incident[] }>('/incidents');

// Change Log
export const fetchChangeLog = () => api.get<{ changes: ChangeEntry[]; entries?: ChangeEntry[] }>('/changes');
export const rollbackChange = (id: string) => api.post(`/changes/${id}/rollback`);

// Node Drain
export const fetchNodeDrainStatus = () => api.get<{ nodes: NodeDrainStatus[] }>('/nodes/drain');
export const drainNode = (node: string) => api.post('/nodes/drain', { node });
export const uncordonNode = (node: string) => api.post('/nodes/uncordon', { node });

// Pod Security
export const fetchPodSecurity = () => api.get<{ reports: PodSecurityReport[] }>('/security/pods');

// Egress Policies
export const fetchEgressPolicies = () => api.get<{ policies: EgressPolicy[] }>('/egress/policies');

// Mesh Services
export const fetchMeshServices = () => api.get<{ services: MeshService[] }>('/servicemesh/services');

// KPR (Kube Proxy Replacement)
export const fetchKPRStatus = () => api.get<KPRStatus>('/kpr/status');

// eBPF Real Data
export const fetchRealEbpfPrograms = () => api.get<{ programs: unknown[]; total: number }>('/ebpf/programs');
export const fetchRealEbpfMaps = () => api.get<{ maps: unknown[]; total: number }>('/ebpf/maps');
export const fetchEbpfProgramStats = (id: string) => api.get(`/ebpf/programs/${id}`);
export const fetchEbpfMapEntries = (id: number, limit?: number) => api.get(`/ebpf/maps/${id}/entries`, { params: { limit: limit || 50 } });
export const fetchEbpfConntrack = () => api.get<{ entries: unknown[]; total: number }>('/ebpf/conntrack');
export const fetchEbpfIpcache = () => api.get<{ entries: unknown[]; total: number }>('/ebpf/ipcache');
export const fetchEbpfLb = () => api.get<{ entries: unknown[]; total: number }>('/ebpf/lb');
export const fetchEbpfDrops = () => api.get<{ drops: unknown[]; total_drops: number }>('/ebpf/drops');
export const fetchEbpfSummary = () => api.get('/ebpf/summary');
