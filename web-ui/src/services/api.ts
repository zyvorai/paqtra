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

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem('cilium-vision-settings');
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
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
  const token = sessionStorage.getItem('cilium-vision-token');
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
      sessionStorage.removeItem('cilium-vision-token');
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface Flow {
  id: string;
  timestamp: string;
  source: FlowEndpoint;
  destination: FlowEndpoint;
  verdict: string;
  protocol: string;
  port: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface FlowStats {
  total_flows: number;
  forwarded: number;
  dropped: number;
  requests_per_second: number;
  avg_latency_ms: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export const fetchPolicies = () =>
  api.get<{ policies: Policy[] }>('/policies');

export const createPolicy = (body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.post('/policies', body);

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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface HeatmapCell {
  source_namespace: string;
  dest_namespace: string;
  flow_count: number;
  dropped_count: number;
  avg_latency: number;

  destination_namespace?: string;
  avg_latency_ms?: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface ZeroTrustScore {
  overall: number;
  network_segmentation: number;
  identity_verification: number;
  encryption: number;
  least_privilege: number;
  monitoring: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  network_interfaces: { name: string; ip: string; mac: string; speed: string; status: string   // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
}

export interface PolicyTemplate {
  id: string;
  name: string;
  category: string;
  description: string;
  yaml: string;
  tags: string[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface DiagnosticTest {
  name: string;
  status: string;
  message: string;
  duration_ms: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface ServiceEdge {
  source: string;
  target: string;
  protocol: string;
  port: number;
  request_rate: number;
  error_rate: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface DnsStats {
  total_queries: number;
  successful: number;
  nxdomain: number;
  servfail: number;
  avg_latency_ms: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface CiliumIdentity {
  id: number;
  labels: string[];
  namespace: string;
  endpoints_count: number;
  policy_count: number;
  created_at: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface CostBreakdown {
  namespace: string;
  cpu_cost: number;
  memory_cost: number;
  network_cost: number;
  storage_cost: number;
  total_cost: number;

  trend?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface CostSummary {
  total_monthly: number;
  total_monthly_cost: number;
  total_daily: number;

  cost_trend?: string;
  savings_potential?: number;
  trend?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface ForecastResult {
  metric: string;
  unit: string;
  recommendation?: string;
  points: { timestamp: string; actual?: number; predicted: number; upper_bound: number; lower_bound: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
}

export interface EncryptionStatus {
  enabled: boolean;
  type: string;
  nodes_encrypted: number;
  nodes_total: number;
  interfaces: { name: string; peer: string; endpoint: string; latest_handshake: string; tx_bytes: number; rx_bytes: number 
  key_rotation?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
  key_rotation_at: string;
}

export interface LBService {
  frontend?: string;
  name: string;
  namespace: string;
  type: string;
  frontend_ip: string;
  frontend_port: number;
  protocol: string;
  backends: { ip: string; port: number; weight: number; state: string   // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface IPAMPool {
  name: string;
  cidr: string;
  total: number;
  allocated: number;
  available: number;

  usage_pct?: number;
  utilization?: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface IPAllocation {
  ip: string;
  pod: string;
  namespace: string;
  node: string;
  pool: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface RBACBinding {
  subject: string;
  subject_kind: string;
  role: string;
  role_kind: string;
  namespace: string;
  permissions: string[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface TroubleshootResult {
  step: string;
  status: string;
  output: string;
  duration_ms: number;

  name?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  timeline: { time: string; event: string 
  duration_minutes?: number;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface NodeDrainStatus {
  node: string;
  status: string;
  pods_evicted: number;
  pods_remaining: number;
  started_at: string;
  cordon: boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

export interface PodSecurityReport {
  namespace: string;
  enforce_level: string;
  audit_level: string;
  warn_level: string;
  total_pods: number;
  compliant_pods: number;
  violations: { pod: string; policy: string; message: string   // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}[];
}

export interface EgressPolicy {
  name: string;
  namespace: string;
  gateway_node: string;
  egress_ip: string;
  destination_cidrs: string[];
  selectors: string | string[];
  status: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any;
}

// ─── Extended API Functions ────────────────────────────────────────────────────

// Events
export const fetchEvents = () => api.get<{ events: K8sEvent[] }>('/events');

// Endpoints
export const fetchEndpoints = () => api.get<{ endpoints: CiliumEndpoint[] }>('/endpoints');

// Nodes
export const fetchNodes = () => api.get<{ nodes: K8sNode[] }>('/nodes');

// Replay / Recordings
export const fetchRecordings = () => api.get<{ recordings: ReplayRecording[] }>('/recordings');
export const startRecording = (body: unknown) => api.post('/recordings', body);
export const stopRecording = (id: string) => api.post(`/recordings/${id}/stop`);
export const fetchRecordingFlows = (id: string) => api.get(`/recordings/${id}/flows`);

// Healer
export const fetchHealerProblems = () => api.get<{ problems: HealerProblem[] }>('/healer/problems');
export const applyHealerFix = (id: string) => api.post(`/healer/problems/${id}/fix`);

// Packet Drops
export const fetchPacketDrops = () => api.get<{ drops: PacketDrop[] }>('/packet-drops');
export const analyzeDrops = (_body?: unknown) => api.post('/packet-drops/analyze');

// Clusters
export const fetchClusters = () => api.get<{ clusters: ClusterInfo[] }>('/clusters');
export const syncClusterPolicies = (name: string) => api.post(`/clusters/${name}/sync`);

// Heatmap
export const fetchHeatmapData = () => api.get<{ cells: HeatmapCell[]; namespaces?: string[] }>('/heatmap');

// Service Dependencies
export const fetchServiceDeps = () => api.get<{ dependencies: ServiceDep[] }>('/service-deps');

// Security
export const fetchSecurityFindings = () => api.get<{ findings: SecurityFinding[] }>('/security/findings');
export const fetchZeroTrustScore = () => api.get<ZeroTrustScore>('/security/zero-trust');

// eBPF
export const fetchEbpfPrograms = () => api.get<{ programs: EbpfProgram[] }>('/ebpf/programs');
export const fetchEbpfMaps = () => api.get<{ maps: EbpfMapInfo[] }>('/ebpf/maps');

// Metrics
export const fetchPrometheusMetrics = () => api.get('/metrics/prometheus');
export const fetchMetricsSummary = () => api.get<MetricsSummary>('/metrics/summary');

// Host
export const fetchHostInfo = () => api.get<HostInfo>('/host');

// Policy Templates
export const fetchPolicyTemplates = () => api.get<{ templates: PolicyTemplate[] }>('/policy-templates');
export const applyTemplate = (id: string, _body?: unknown) => api.post(`/policy-templates/${id}/apply`);

// Diagnostics
export const runDiagnostics = () => api.post<{ tests: DiagnosticTest[] }>('/diagnostics');
export const fetchConnectivityTest = () => api.get('/diagnostics/connectivity');

// Audit
export const fetchAuditLog = () => api.get<{ entries: AuditEntry[] }>('/audit');

// Alerts
export const fetchAlertRules = () => api.get<{ rules: AlertRule[] }>('/alerts/rules');
export const fetchAlertHistory = () => api.get<{ alerts: AlertEvent[]; events?: AlertEvent[] }>('/alerts/history');
export const toggleAlertRule = (id: string) => api.post(`/alerts/rules/${id}/toggle`);

// Service Map
export const fetchServiceMap = () => api.get<{ nodes: ServiceNode[]; edges: ServiceEdge[] }>('/service-map');

// Packet Capture
export const fetchCaptureSessions = () => api.get<{ sessions: CaptureSession[] }>('/captures');
export const startCapture = (body: unknown) => api.post('/captures', body);
export const stopCapture = (id: string) => api.post(`/captures/${id}/stop`);

// DNS
export const fetchDnsQueries = (params?: { namespace?: string; query_name?: string }) =>
  api.get<{ queries: DnsQuery[] }>('/dns/queries', { params });
export const fetchDnsStats = () => api.get<DnsStats>('/dns/stats');

// Identities
export const fetchIdentities = () => api.get<{ identities: CiliumIdentity[] }>('/identities');

// Cluster Mesh
export const fetchMeshPeers = () => api.get<{ peers: MeshPeer[] }>('/mesh/peers');
export const connectMeshPeer = (body: unknown) => api.post('/mesh/peers', body);

// BGP
export const fetchBgpPeers = () => api.get<{ peers: BgpPeer[] }>('/bgp/peers');

// Bandwidth
export const fetchBandwidthData = () => api.get<{ entries: BandwidthEntry[] }>('/bandwidth');

// Cost
export const fetchCostBreakdown = () => api.get<{ breakdown: CostBreakdown[]; summary: CostSummary }>('/cost');

// Forecast
export const fetchForecast = (body: unknown) => api.post<ForecastResult>('/forecast', body);
export const fetchForecastMetrics = () => api.get<{ metrics: string[] }>('/forecast/metrics');

// Encryption
export const fetchEncryptionStatus = () => api.get<EncryptionStatus>('/encryption');

// Load Balancing
export const fetchLBServices = () => api.get<{ services: LBService[] }>('/lb/services');

// Ingress
export const fetchIngressRoutes = () => api.get<{ routes: IngressRoute[] }>('/ingress');

// IPAM
export const fetchIPAMPools = () => api.get<{ pools: IPAMPool[] }>('/ipam/pools');
export const fetchIPAllocations = () => api.get<{ allocations: IPAllocation[] }>('/ipam/allocations');

// Latency
export const fetchLatencyAnalysis = () => api.get<{ services: LatencyBreakdown[] }>('/latency');

// Mirror
export const fetchMirrorRules = () => api.get<{ rules: MirrorRule[] }>('/mirror');
export const createMirrorRule = (body: unknown) => api.post('/mirror', body);
export const deleteMirrorRule = (id: string) => api.delete(`/mirror/${id}`);

// Cluster Health
export const fetchClusterHealth = () => api.get<ClusterHealthSummary>('/cluster/health');

// RBAC
export const fetchRBACBindings = () => api.get<{ bindings: RBACBinding[] }>('/rbac');

// Network Interfaces
export const fetchNetInterfaces = () => api.get<{ interfaces: NetInterface[] }>('/interfaces');

// Troubleshoot
export const runTroubleshoot = (body: unknown) => api.post<{ results: TroubleshootResult[] }>('/troubleshoot', body);

// WireGuard
export const fetchWireGuardPeers = () => api.get<{ peers: WireGuardPeer[] }>('/wireguard/peers');

// Cilium Status
export const fetchCiliumStatus = () => api.get<{ agents: CiliumAgentStatus[] }>('/cilium/status');

// Policy Validation
export const validatePolicy = (yaml: string) => api.post('/policies/validate', { yaml });

// Export
export const fetchExportConfigs = () => api.get<{ configs: ExportConfig[] }>('/export');
export const createExportConfig = (body: unknown) => api.post('/export', body);
export const deleteExportConfig = (id: string) => api.delete(`/export/${id}`);

// SLOs
export const fetchSLOs = () => api.get<{ slos: SLOTarget[] }>('/slos');

// Incidents
export const fetchIncidents = () => api.get<{ incidents: Incident[] }>('/incidents');

// Change Log
export const fetchChangeLog = () => api.get<{ changes: ChangeEntry[]; entries?: ChangeEntry[] }>('/changelog');
export const rollbackChange = (id: string) => api.post(`/changelog/${id}/rollback`);

// Node Drain
export const fetchNodeDrainStatus = () => api.get<{ nodes: NodeDrainStatus[] }>('/nodes/drain');
export const drainNode = (node: string) => api.post(`/nodes/${node}/drain`);
export const uncordonNode = (node: string) => api.post(`/nodes/${node}/uncordon`);

// Pod Security
export const fetchPodSecurity = () => api.get<{ reports: PodSecurityReport[] }>('/pod-security');

// Egress Policies
export const fetchEgressPolicies = () => api.get<{ policies: EgressPolicy[] }>('/egress-policies');

// Mesh Services
export const fetchMeshServices = () => api.get<{ services: MeshService[] }>('/mesh/services');

// KPR (Kube Proxy Replacement)
export const fetchKPRStatus = () => api.get<KPRStatus>('/kpr');
