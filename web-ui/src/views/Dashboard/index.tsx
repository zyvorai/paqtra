import React, { useEffect, useState, useRef, useCallback } from 'react';
import {
  Activity,
  Gauge,
  AlertTriangle,
  CheckCircle,
  Wifi,
  WifiOff,
  TrendingUp,
  Cpu,
  Network,
  Shield,
  Eye,
  GitBranch,
  Map,
  BarChart3,
  Server,
  HardDrive,
  Clock,
  Layers,
} from 'lucide-react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  AreaChart,
  Area,
  Legend,
} from 'recharts';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useMetricsHistory } from '../../hooks/useMetricsHistory';
import { QuickLinks } from '../../components/QuickLinks';
import { HealthCheckCard } from '../../components/HealthCheckCard';
import { PipelineView } from '../../components/PipelineView';
import { SystemInfoPanel } from '../../components/SystemInfoPanel';
import { ActivityFeed } from '../../components/ActivityFeed';
import { NamespaceSidebar } from '../../components/NamespaceSidebar';
import {
  fetchNodes,
  fetchEndpoints,
  fetchEvents,
  fetchHostInfo,
  fetchClusters,
  fetchClusterHealth,
  fetchCiliumStatus,
} from '../../services/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface MetricData {
  timestamp: string;
  requests_per_sec: number;
  avg_latency_ms: number;
  error_rate: number;
}

interface HealthCheck {
  name: string;
  status: 'healthy' | 'degraded' | 'unhealthy' | 'unknown';
  detail?: string;
}

interface PipelineStage {
  name: string;
  count: number;
  color: string;
  active?: boolean;
}

interface SystemInfoItem {
  label: string;
  value: string;
  icon?: React.ReactNode;
}

interface FeedEvent {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  message: string;
  timestamp: string;
  source?: string;
}

interface NamespaceInfo {
  name: string;
  pods: number;
  endpoints: number;
  policies: number;
  status?: 'active' | 'warning' | 'error';
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getWsUrl(path: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}${path}`;
}

const RECONNECT_DELAY_MS = 3000;

const DARK_TOOLTIP_STYLE = {
  backgroundColor: '#0f172a',
  border: '1px solid #1e293b',
  borderRadius: '8px',
  fontSize: '12px',
  color: '#e2e8f0',
};

function formatXAxisTime(value: string) {
  const date = new Date(value);
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

// ---------------------------------------------------------------------------
// Quick Links config
// ---------------------------------------------------------------------------

const QUICK_LINKS = [
  { title: 'Flows', description: 'Monitor network flows', icon: <Network className="w-5 h-5" />, path: '/flows' },
  { title: 'Topology', description: 'Network topology map', icon: <GitBranch className="w-5 h-5" />, path: '/topology' },
  { title: 'Policies', description: 'Security policies', icon: <Shield className="w-5 h-5" />, path: '/policies' },
  { title: 'Service Map', description: 'Service dependencies', icon: <Map className="w-5 h-5" />, path: '/servicemap' },
  { title: 'Anomalies', description: 'Threat detection', icon: <Eye className="w-5 h-5" />, path: '/anomalies' },
  { title: 'Metrics', description: 'System metrics', icon: <BarChart3 className="w-5 h-5" />, path: '/metrics' },
];

// ---------------------------------------------------------------------------
// Fallback / demo data
// ---------------------------------------------------------------------------

const FALLBACK_HEALTH_CHECKS: HealthCheck[] = [
  { name: 'Cilium Agent', status: 'healthy', detail: 'All agents running' },
  { name: 'Hubble Relay', status: 'healthy', detail: 'Relay connected' },
  { name: 'CoreDNS', status: 'healthy', detail: 'DNS resolution OK' },
  { name: 'Endpoints', status: 'healthy', detail: 'All endpoints ready' },
  { name: 'Policies', status: 'healthy', detail: 'No policy conflicts' },
  { name: 'eBPF Maps', status: 'healthy', detail: 'Maps loaded' },
];

function buildSystemInfoItems(data: {
  clusterName: string;
  ciliumVersion: string;
  nodeCount: number;
  podCount: number;
  hubbleStatus: string;
  kernelVersion: string;
}): SystemInfoItem[] {
  return [
    { label: 'Cluster', value: data.clusterName, icon: <Layers className="w-3.5 h-3.5" /> },
    { label: 'Cilium', value: data.ciliumVersion, icon: <Shield className="w-3.5 h-3.5" /> },
    { label: 'Nodes', value: String(data.nodeCount), icon: <Server className="w-3.5 h-3.5" /> },
    { label: 'Pods', value: String(data.podCount), icon: <HardDrive className="w-3.5 h-3.5" /> },
    { label: 'Hubble', value: data.hubbleStatus, icon: <Clock className="w-3.5 h-3.5" /> },
    { label: 'Kernel', value: data.kernelVersion, icon: <Cpu className="w-3.5 h-3.5" /> },
  ];
}

const FALLBACK_SYSTEM_DATA = {
  clusterName: 'production',
  ciliumVersion: '1.15.4',
  nodeCount: 3,
  podCount: 42,
  hubbleStatus: 'Connected',
  kernelVersion: '6.1.0',
};

const FALLBACK_NAMESPACES: NamespaceInfo[] = [
  { name: 'default', pods: 4, endpoints: 4, policies: 2, status: 'active' },
  { name: 'kube-system', pods: 12, endpoints: 12, policies: 5, status: 'active' },
  { name: 'cilium', pods: 6, endpoints: 6, policies: 3, status: 'active' },
  { name: 'monitoring', pods: 3, endpoints: 3, policies: 1, status: 'active' },
  { name: 'app-prod', pods: 8, endpoints: 8, policies: 4, status: 'active' },
  { name: 'app-staging', pods: 5, endpoints: 5, policies: 2, status: 'warning' },
];

function makeDemoActivity(): FeedEvent[] {
  const now = Date.now();
  return [
    { id: '1', timestamp: new Date(now - 5000).toISOString(), type: 'info', message: 'HTTP GET /api/v1/health from frontend-7b9d to backend-3c4a', source: 'app-prod' },
    { id: '2', timestamp: new Date(now - 12000).toISOString(), type: 'success', message: 'CiliumNetworkPolicy "allow-dns" applied', source: 'kube-system' },
    { id: '3', timestamp: new Date(now - 30000).toISOString(), type: 'error', message: 'Packet dropped: policy denied egress to 10.0.0.5:443', source: 'app-staging' },
    { id: '4', timestamp: new Date(now - 45000).toISOString(), type: 'info', message: 'TCP SYN from worker-1 to redis-master:6379', source: 'default' },
    { id: '5', timestamp: new Date(now - 60000).toISOString(), type: 'warning', message: 'Anomaly detected: unusual DNS query volume', source: 'monitoring' },
    { id: '6', timestamp: new Date(now - 90000).toISOString(), type: 'info', message: 'Hubble relay reconnected after brief interruption' },
    { id: '7', timestamp: new Date(now - 120000).toISOString(), type: 'success', message: 'gRPC call ordersvc.OrderService/GetOrder completed 23ms', source: 'app-prod' },
    { id: '8', timestamp: new Date(now - 180000).toISOString(), type: 'info', message: 'Identity 12849 resolved for pod metrics-collector', source: 'monitoring' },
  ];
}

// ---------------------------------------------------------------------------
// Dashboard Component
// ---------------------------------------------------------------------------

const Dashboard: React.FC = () => {
  usePageTitle('Dashboard');
  const { history, addMetrics } = useMetricsHistory<MetricData>(60);
  const [status, setStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
  const wsRef = useRef<WebSocket | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  // New state for added sections
  const [healthChecks, setHealthChecks] = useState<HealthCheck[]>(FALLBACK_HEALTH_CHECKS);
  const [systemData, setSystemData] = useState(FALLBACK_SYSTEM_DATA);
  const [namespaces, setNamespaces] = useState<NamespaceInfo[]>(FALLBACK_NAMESPACES);
  const [activityEvents, setActivityEvents] = useState<FeedEvent[]>(makeDemoActivity);
  const [pipelineStages, setPipelineStages] = useState<PipelineStage[]>([
    { name: 'Ingress', count: 0, color: 'blue' },
    { name: 'Policy Check', count: 0, color: 'purple' },
    { name: 'Forwarded', count: 0, color: 'green' },
    { name: 'Dropped', count: 0, color: 'red' },
    { name: 'Egress', count: 0, color: 'cyan' },
  ]);

  // Cumulative pipeline counters kept in a ref so ws handler stays stable
  const pipelineCounts = useRef({ ingress: 0, policy: 0, forwarded: 0, dropped: 0, egress: 0 });

  // -----------------------------------------------------------------------
  // WebSocket (existing logic preserved)
  // -----------------------------------------------------------------------

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;
    setStatus('connecting');
    const ws = new WebSocket(getWsUrl('/api/v1/ws/metrics'));
    wsRef.current = ws;

    ws.onopen = () => { if (mountedRef.current) setStatus('connected'); };
    ws.onmessage = (event) => {
      if (!mountedRef.current) return;
      try {
        const data = JSON.parse(event.data);
        if (data.timestamp) {
          addMetrics(data);

          // Update pipeline counts from incoming metrics
          const rps = data.requests_per_sec ?? 0;
          const errRate = data.error_rate ?? 0;
          const dropped = Math.round(rps * errRate);
          const forwarded = Math.max(0, Math.round(rps) - dropped);

          pipelineCounts.current.ingress += Math.round(rps);
          pipelineCounts.current.policy += Math.round(rps);
          pipelineCounts.current.forwarded += forwarded;
          pipelineCounts.current.dropped += dropped;
          pipelineCounts.current.egress += forwarded;

          setPipelineStages([
            { name: 'Ingress', count: pipelineCounts.current.ingress, color: 'blue', active: true },
            { name: 'Policy Check', count: pipelineCounts.current.policy, color: 'purple', active: true },
            { name: 'Forwarded', count: pipelineCounts.current.forwarded, color: 'green', active: true },
            { name: 'Dropped', count: pipelineCounts.current.dropped, color: 'red', active: pipelineCounts.current.dropped > 0 },
            { name: 'Egress', count: pipelineCounts.current.egress, color: 'cyan', active: true },
          ]);
        }
      } catch { /* ignore */ }
    };
    ws.onerror = () => { if (mountedRef.current) setStatus('disconnected'); };
    ws.onclose = () => {
      if (mountedRef.current) setStatus('disconnected');
      wsRef.current = null;
      if (mountedRef.current) {
        timerRef.current = setTimeout(connect, RECONNECT_DELAY_MS);
      }
    };
  }, [addMetrics]);

  useEffect(() => {
    mountedRef.current = true;
    connect();
    return () => {
      mountedRef.current = false;
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      wsRef.current?.close();
    };
  }, [connect]);

  // -----------------------------------------------------------------------
  // Fetch API data on mount (with fallback)
  // -----------------------------------------------------------------------

  useEffect(() => {
    let cancelled = false;

    async function loadApiData() {
      // Cluster health -> health checks
      try {
        const res = await fetchClusterHealth();
        if (!cancelled && res.data?.components?.length) {
          const checks: HealthCheck[] = res.data.components.map((c) => ({
            name: c.name,
            status: (c.status === 'ok' || c.status === 'healthy') ? 'healthy' as const
              : c.status === 'degraded' ? 'degraded' as const
              : c.status === 'unhealthy' ? 'unhealthy' as const
              : 'unknown' as const,
            detail: c.message,
          }));
          setHealthChecks(checks);
        }
      } catch { /* use fallback */ }

      // Clusters + nodes + host info -> system info
      try {
        const [clustersRes, nodesRes, hostRes] = await Promise.allSettled([
          fetchClusters(),
          fetchNodes(),
          fetchHostInfo(),
        ]);

        if (!cancelled) {
          const info = { ...FALLBACK_SYSTEM_DATA };

          if (clustersRes.status === 'fulfilled' && clustersRes.value.data?.clusters?.length) {
            const cl = clustersRes.value.data.clusters[0];
            info.clusterName = cl.name;
            info.ciliumVersion = cl.cilium_version || info.ciliumVersion;
            info.podCount = cl.pods || info.podCount;
            info.nodeCount = cl.nodes || info.nodeCount;
          }

          if (nodesRes.status === 'fulfilled' && nodesRes.value.data?.nodes?.length) {
            const nodes = nodesRes.value.data.nodes;
            info.nodeCount = nodes.length;
            info.kernelVersion = nodes[0]?.kernel || info.kernelVersion;
          }

          if (hostRes.status === 'fulfilled') {
            const h = hostRes.value.data;
            info.kernelVersion = h.kernel || info.kernelVersion;
          }

          setSystemData(info);
        }
      } catch { /* use fallback */ }

      // Cilium agent status -> update system data
      try {
        const res = await fetchCiliumStatus();
        if (!cancelled && res.data?.agents?.length) {
          const agent = res.data.agents[0];
          setSystemData((prev) => ({
            ...prev,
            ciliumVersion: agent.version || prev.ciliumVersion,
            hubbleStatus: agent.status === 'OK' || agent.status === 'ok' ? 'Connected' : agent.status,
          }));
        }
      } catch { /* use fallback */ }

      // Endpoints -> namespace sidebar
      try {
        const res = await fetchEndpoints();
        if (!cancelled && res.data?.endpoints?.length) {
          const nsMap: Record<string, { pods: number; endpoints: number }> = {};
          for (const ep of res.data.endpoints) {
            const ns = ep.namespace || 'default';
            if (!nsMap[ns]) nsMap[ns] = { pods: 0, endpoints: 0 };
            nsMap[ns].endpoints += 1;
            nsMap[ns].pods += 1;
          }
          const nsList: NamespaceInfo[] = Object.entries(nsMap).map(([name, data]) => ({
            name,
            pods: data.pods,
            endpoints: data.endpoints,
            policies: 0,
            status: 'active' as const,
          }));
          if (nsList.length > 0) setNamespaces(nsList);
        }
      } catch { /* use fallback */ }

      // Events -> activity feed
      try {
        const res = await fetchEvents();
        if (!cancelled && res.data?.events?.length) {
          const items: FeedEvent[] = res.data.events.slice(0, 10).map((ev) => ({
            id: ev.id,
            timestamp: ev.last_timestamp || ev.first_timestamp,
            type: ev.type === 'Warning' ? 'warning' as const
              : ev.type === 'Error' ? 'error' as const
              : ev.reason?.toLowerCase().includes('success') ? 'success' as const
              : 'info' as const,
            message: ev.message,
            source: ev.namespace,
          }));
          if (items.length > 0) setActivityEvents(items);
        }
      } catch { /* use fallback */ }
    }

    loadApiData();
    return () => { cancelled = true; };
  }, []);

  // -----------------------------------------------------------------------
  // Derived values
  // -----------------------------------------------------------------------

  const latest = history[history.length - 1];
  const prev = history[history.length - 2];

  const trend = (curr?: number, old?: number) => {
    if (!curr || !old || old === 0) return null;
    return ((curr - old) / old) * 100;
  };

  const reqTrend = trend(latest?.requests_per_sec, prev?.requests_per_sec);
  const latTrend = trend(latest?.avg_latency_ms, prev?.avg_latency_ms);

  const hasConnectionIssue = status === 'disconnected';

  const systemInfoItems = buildSystemInfoItems(systemData);

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  return (
    <div className="space-y-6">
      {/* ── Header ────────────────────────────────────────────────────── */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gradient-blue">Cilium Vision</h1>
          <p className="text-sm text-slate-400 mt-1">
            {status === 'connected'
              ? 'Real-time monitoring active'
              : status === 'connecting'
                ? 'Connecting...'
                : 'Disconnected'}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {status === 'connected' ? (
            <>
              <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
              <Wifi className="w-4 h-4 text-emerald-400" />
              <span className="text-sm font-medium text-emerald-400">Connected</span>
            </>
          ) : status === 'connecting' ? (
            <>
              <span className="w-2.5 h-2.5 rounded-full bg-amber-500 animate-pulse" />
              <span className="text-sm font-medium text-amber-400">Connecting...</span>
            </>
          ) : (
            <>
              <WifiOff className="w-4 h-4 text-red-400" />
              <span className="text-sm font-medium text-red-400">Disconnected</span>
            </>
          )}
        </div>
      </div>

      {/* ── Connection Status Banner ──────────────────────────────────── */}
      {hasConnectionIssue && (
        <div className="bg-red-500/10 rounded-xl border border-red-500/30 p-4">
          <div className="flex items-center gap-3">
            <WifiOff className="w-5 h-5 text-red-400 flex-shrink-0" />
            <div>
              <p className="text-sm font-semibold text-red-400">
                WebSocket Connection Failed - Real-time updates unavailable
              </p>
              <p className="text-xs text-red-400/70 mt-1">
                Reconnecting...
              </p>
            </div>
          </div>
        </div>
      )}

      {status === 'connected' && (
        <div className="stat-card-green rounded-xl border border-slate-700/50 px-5 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
            <Wifi className="w-4 h-4 text-emerald-400" />
            <span className="text-sm font-medium text-emerald-400">Connected</span>
          </div>
          <span className="text-sm text-slate-400">
            {history.length} data points collected
          </span>
        </div>
      )}

      {/* ── Primary Stat Cards ────────────────────────────────────────── */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Requests/sec */}
        <div className="stat-card-blue rounded-xl border border-slate-700/50 p-5 card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <Gauge className="h-5 w-5 text-white" />
            </div>
            <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400">
              live
            </span>
          </div>
          <div className="text-2xl font-bold text-white">
            {(latest?.requests_per_sec ?? 0).toFixed(1)}
          </div>
          <div className="text-xs text-slate-400 mt-1">Requests/sec</div>
          {reqTrend != null && (
            <div className={`flex items-center gap-1 mt-1 text-xs font-semibold ${reqTrend > 0 ? 'text-emerald-400' : 'text-red-400'}`}>
              <TrendingUp className="w-3 h-3" />
              {Math.abs(reqTrend).toFixed(1)}%
            </div>
          )}
        </div>

        {/* Avg Latency */}
        <div className="stat-card-purple rounded-xl border border-slate-700/50 p-5 card-glow-purple transition-all hover:scale-[1.02]">
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-purple-500 to-purple-700 flex items-center justify-center shadow-lg shadow-purple-500/20">
              <Activity className="h-5 w-5 text-white" />
            </div>
            <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-purple-500/10 text-purple-400">
              avg
            </span>
          </div>
          <div className="text-2xl font-bold text-white">
            {(latest?.avg_latency_ms ?? 0).toFixed(1)} ms
          </div>
          <div className="text-xs text-slate-400 mt-1">Avg Latency</div>
          {latTrend != null && (
            <div className={`flex items-center gap-1 mt-1 text-xs font-semibold ${latTrend < 0 ? 'text-emerald-400' : 'text-red-400'}`}>
              <TrendingUp className="w-3 h-3" />
              {Math.abs(latTrend).toFixed(1)}%
            </div>
          )}
        </div>

        {/* Error Rate */}
        <div className="stat-card-red rounded-xl border border-slate-700/50 p-5 card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
              <AlertTriangle className="h-5 w-5 text-white" />
            </div>
            <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-red-500/10 text-red-400">
              errors
            </span>
          </div>
          <div className="text-2xl font-bold text-white">
            {((latest?.error_rate ?? 0) * 100).toFixed(2)}%
          </div>
          <div className="text-xs text-slate-400 mt-1">Error Rate</div>
        </div>

        {/* Status */}
        <div className="stat-card-green rounded-xl border border-slate-700/50 p-5 card-glow-green transition-all hover:scale-[1.02]">
          <div className="flex items-center justify-between mb-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-green-500/20">
              <CheckCircle className="h-5 w-5 text-white" />
            </div>
            <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-green-500/10 text-green-400">
              health
            </span>
          </div>
          <div className={`text-2xl font-bold ${status === 'connected' ? 'text-emerald-400' : 'text-slate-400'}`}>
            {status === 'connected' ? 'Healthy' : 'Connecting...'}
          </div>
          <div className="text-xs text-slate-400 mt-1">Status</div>
        </div>
      </div>

      {/* ── Flow Pipeline ─────────────────────────────────────────────── */}
      <PipelineView stages={pipelineStages} />

      {/* ── Health Checks + System Info (2 col + 1 col) ───────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Health Check Grid - takes 2 cols */}
        <div className="lg:col-span-2">
          <HealthCheckCard checks={healthChecks} />
        </div>

        {/* System Info Panel - takes 1 col */}
        <div className="lg:col-span-1">
          <SystemInfoPanel info={systemInfoItems} title="System Info" />
        </div>
      </div>

      {/* ── Quick Links + Namespace Sidebar (2 col + 1 col) ──────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Quick Links - takes 2 cols */}
        <div className="lg:col-span-2">
          <QuickLinks links={QUICK_LINKS} />
        </div>

        {/* Namespace Sidebar - takes 1 col */}
        <div className="lg:col-span-1">
          <NamespaceSidebar namespaces={namespaces} />
        </div>
      </div>

      {/* ── Charts: Request Rate | Latency (side-by-side) ─────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Request Rate Chart */}
        <div className="bg-slate-800/50 rounded-xl p-5 border border-slate-700/50">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <TrendingUp className="w-4 h-4 text-white" />
            </div>
            <h3 className="text-base font-semibold text-white">Request Rate Over Time</h3>
          </div>
          {history.length === 0 ? (
            <div className="h-[300px] flex items-center justify-center text-slate-500">
              No data available
            </div>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={history}>
                <defs>
                  <linearGradient id="gradReq" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="timestamp" stroke="#475569" fontSize={11} tickFormatter={formatXAxisTime} />
                <YAxis stroke="#475569" fontSize={11} />
                <Tooltip contentStyle={DARK_TOOLTIP_STYLE} />
                <Legend />
                <Area type="monotone" dataKey="requests_per_sec" stroke="#3b82f6" strokeWidth={2} fill="url(#gradReq)" dot={false} name="Req/s" />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Latency Chart */}
        <div className="bg-slate-800/50 rounded-xl p-5 border border-slate-700/50">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-purple-700 flex items-center justify-center shadow-lg shadow-purple-500/20">
              <Activity className="w-4 h-4 text-white" />
            </div>
            <h3 className="text-base font-semibold text-white">Latency Over Time</h3>
          </div>
          {history.length === 0 ? (
            <div className="h-[300px] flex items-center justify-center text-slate-500">
              No data available
            </div>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={history}>
                <defs>
                  <linearGradient id="gradLatency" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#a855f7" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#a855f7" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="timestamp" stroke="#475569" fontSize={11} tickFormatter={formatXAxisTime} />
                <YAxis stroke="#475569" fontSize={11} />
                <Tooltip contentStyle={DARK_TOOLTIP_STYLE} />
                <Legend />
                <Area type="monotone" dataKey="avg_latency_ms" stroke="#a855f7" strokeWidth={2} fill="url(#gradLatency)" dot={false} name="Latency (ms)" />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </div>
      </div>

      {/* ── Error Rate Chart + Activity Feed (side-by-side) ───────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Error Rate Chart */}
        <div className="bg-slate-800/50 rounded-xl p-5 border border-slate-700/50">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
              <AlertTriangle className="w-4 h-4 text-white" />
            </div>
            <h3 className="text-base font-semibold text-white">Error Rate Over Time</h3>
          </div>
          {history.length === 0 ? (
            <div className="h-[300px] flex items-center justify-center text-slate-500">
              No data available
            </div>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={history}>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="timestamp" stroke="#475569" fontSize={11} tickFormatter={formatXAxisTime} />
                <YAxis stroke="#475569" fontSize={11} />
                <Tooltip contentStyle={DARK_TOOLTIP_STYLE} />
                <Line type="monotone" dataKey="error_rate" stroke="#ef4444" dot={false} strokeWidth={2} name="Error Rate" />
              </LineChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Recent Activity Feed */}
        <ActivityFeed events={activityEvents} maxItems={8} title="Recent Activity" />
      </div>
    </div>
  );
};

export default Dashboard;
