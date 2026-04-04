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
  Zap,
  BarChart3,
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

interface MetricData {
  timestamp: string;
  requests_per_sec: number;
  avg_latency_ms: number;
  error_rate: number;
}

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

const QUICK_LINKS = [
  { title: 'Flows', description: 'Monitor network flows', icon: <Network className="w-5 h-5" />, path: '/flows' },
  { title: 'Topology', description: 'Network topology map', icon: <GitBranch className="w-5 h-5" />, path: '/topology' },
  { title: 'Policies', description: 'Security policies', icon: <Shield className="w-5 h-5" />, path: '/policies' },
  { title: 'Service Map', description: 'Service dependencies', icon: <Map className="w-5 h-5" />, path: '/servicemap' },
  { title: 'Anomalies', description: 'Threat detection', icon: <Eye className="w-5 h-5" />, path: '/anomalies' },
  { title: 'Metrics', description: 'System metrics', icon: <BarChart3 className="w-5 h-5" />, path: '/metrics' },
];

const Dashboard: React.FC = () => {
  usePageTitle('Dashboard');
  const { history, addMetrics } = useMetricsHistory<MetricData>(60);
  const [status, setStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
  const wsRef = useRef<WebSocket | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;
    setStatus('connecting');
    const ws = new WebSocket(getWsUrl('/api/v1/ws/metrics'));
    wsRef.current = ws;

    ws.onopen = () => setStatus('connected');
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.timestamp) {
          addMetrics(data);
        }
      } catch { /* ignore */ }
    };
    ws.onerror = () => setStatus('disconnected');
    ws.onclose = () => {
      setStatus('disconnected');
      wsRef.current = null;
      timerRef.current = setTimeout(connect, RECONNECT_DELAY_MS);
    };
  }, [addMetrics]);

  useEffect(() => {
    connect();
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const latest = history[history.length - 1];
  const prev = history[history.length - 2];

  const trend = (curr?: number, old?: number) => {
    if (!curr || !old || old === 0) return null;
    return ((curr - old) / old) * 100;
  };

  const reqTrend = trend(latest?.requests_per_sec, prev?.requests_per_sec);
  const latTrend = trend(latest?.avg_latency_ms, prev?.avg_latency_ms);

  const hasConnectionIssue = status === 'disconnected';

  return (
    <div className="space-y-6">
      {/* Header */}
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

      {/* Connection Status Banner */}
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

      {/* Quick Links */}
      <QuickLinks links={QUICK_LINKS} />

      {/* Stat Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Requests/sec - Blue */}
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

        {/* Avg Latency - Purple */}
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

        {/* Error Rate - Red */}
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

        {/* Status - Green */}
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

      {/* Secondary Stats */}
      <div className="grid grid-cols-2 gap-4">
        <div className="stat-card-cyan rounded-xl p-5 border border-slate-700/50 card-glow-cyan transition-all hover:scale-[1.01]">
          <div className="flex items-center gap-2 mb-2">
            <Cpu className="w-4 h-4 text-cyan-400" />
            <span className="text-sm text-slate-400">Data Points</span>
          </div>
          <div className="text-xl font-bold text-white">
            {history.length}
          </div>
        </div>
        <div className="stat-card-orange rounded-xl p-5 border border-slate-700/50 card-glow transition-all hover:scale-[1.01]">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="w-4 h-4 text-orange-400" />
            <span className="text-sm text-slate-400">Connection Status</span>
          </div>
          <div className={`text-xl font-bold ${status === 'connected' ? 'text-emerald-400' : 'text-slate-400'}`}>
            {status}
          </div>
        </div>
      </div>

      {/* Charts */}
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
                  <linearGradient id="gradError" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#ef4444" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
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

      {/* Error Rate Chart */}
      <div className="bg-slate-800/50 rounded-xl p-5 border border-slate-700/50">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
            <AlertTriangle className="w-4 h-4 text-white" />
          </div>
          <h3 className="text-base font-semibold text-white">Error Rate Over Time</h3>
        </div>
        {history.length === 0 ? (
          <div className="h-[200px] flex items-center justify-center text-slate-500">
            No data available
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={200}>
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
    </div>
  );
};

export default Dashboard;
