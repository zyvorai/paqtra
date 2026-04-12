import React, { useState, useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import {
  Search,
  ArrowRight,
  Ban,
  Eye,
  Gauge,
  ArrowUpRight,
  ArrowDownRight,
  Loader2,
  SlidersHorizontal,
  ChevronDown,
  ChevronUp,
  Shuffle,
} from 'lucide-react';
import { PieChart, Pie, Cell, ResponsiveContainer } from 'recharts';
import { fetchFlows as apiFetchFlows, fetchFlowStats as apiFetchFlowStats, Flow, FlowStats } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { usePagination } from '../../hooks/usePagination';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import { useNamespaceStore } from '../../stores/namespaceStore';

type Verdict = 'ALL' | 'FORWARDED' | 'DROPPED' | 'AUDIT' | 'REDIRECTED';
type Protocol = 'ALL' | 'TCP' | 'UDP' | 'ICMP' | 'SCTP';
type TimeRange = 'ALL' | '5m' | '15m' | '1h' | '6h' | '24h';
type HttpMethod = 'ALL' | 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';

const TIME_RANGE_LABELS: Record<TimeRange, string> = {
  ALL: 'All Time',
  '5m': 'Last 5m',
  '15m': 'Last 15m',
  '1h': 'Last 1h',
  '6h': 'Last 6h',
  '24h': 'Last 24h',
};

const TIME_RANGE_MS: Record<TimeRange, number> = {
  ALL: 0,
  '5m': 5 * 60 * 1000,
  '15m': 15 * 60 * 1000,
  '1h': 60 * 60 * 1000,
  '6h': 6 * 60 * 60 * 1000,
  '24h': 24 * 60 * 60 * 1000,
};

const VERDICT_BADGE: Record<string, string> = {
  FORWARDED: 'bg-green-500/15 text-green-400 border-green-500/30',
  DROPPED: 'bg-red-500/15 text-red-400 border-red-500/30',
  AUDIT: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  REDIRECTED: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
};

const VERDICT_ICON: Record<string, React.ReactNode> = {
  FORWARDED: <ArrowRight className="w-3 h-3" />,
  DROPPED: <Ban className="w-3 h-3" />,
  AUDIT: <Eye className="w-3 h-3" />,
  REDIRECTED: <Shuffle className="w-3 h-3" />,
};

const DONUT_COLORS: Record<string, string> = {
  Forwarded: '#22c55e',
  Dropped: '#ef4444',
  Redirected: '#3b82f6',
};

/** Parse a port filter string into a matcher. Supports single port ("80") or range ("8080-8090"). */
function parsePortFilter(value: string): ((port: number) => boolean) | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const rangeMatch = trimmed.match(/^(\d+)\s*-\s*(\d+)$/);
  if (rangeMatch) {
    const lo = parseInt(rangeMatch[1], 10);
    const hi = parseInt(rangeMatch[2], 10);
    if (isNaN(lo) || isNaN(hi)) return null;
    return (p) => p >= lo && p <= hi;
  }
  const single = parseInt(trimmed, 10);
  if (isNaN(single)) return null;
  return (p) => p === single;
}

/** Match an HTTP status filter like "500", "4xx", "5xx" */
function parseHttpStatusFilter(value: string): ((status: number) => boolean) | null {
  const trimmed = value.trim().toLowerCase();
  if (!trimmed) return null;
  const classMatch = trimmed.match(/^([1-5])xx$/);
  if (classMatch) {
    const base = parseInt(classMatch[1], 10) * 100;
    return (s) => s >= base && s < base + 100;
  }
  const exact = parseInt(trimmed, 10);
  if (isNaN(exact)) return null;
  return (s) => s === exact;
}

function httpStatusColor(code: number): string {
  if (code >= 200 && code < 300) return 'text-green-400';
  if (code >= 300 && code < 400) return 'text-blue-400';
  if (code >= 400 && code < 500) return 'text-orange-400';
  if (code >= 500) return 'text-red-400';
  return 'text-slate-400';
}

function httpStatusBg(code: number): string {
  if (code >= 200 && code < 300) return 'bg-green-500/15 border-green-500/30';
  if (code >= 300 && code < 400) return 'bg-blue-500/15 border-blue-500/30';
  if (code >= 400 && code < 500) return 'bg-orange-500/15 border-orange-500/30';
  if (code >= 500) return 'bg-red-500/15 border-red-500/30';
  return 'bg-slate-700/50 border-slate-700/50';
}

function truncateUrl(s: string, max: number): string {
  return s.length > max ? s.slice(0, max) + '\u2026' : s;
}

const Flows: React.FC = () => {
  usePageTitle('Flows');
  const { selectedNamespace } = useNamespaceStore();
  const [searchParams] = useSearchParams();
  const [namespace, setNamespace] = useState(searchParams.get('namespace') ?? selectedNamespace);
  const [verdict, setVerdict] = useState<Verdict>('ALL');
  const [searchText, setSearchText] = useState(searchParams.get('search') ?? '');
  const pagination = usePagination({ initialLimit: 25 });
  const [flows, setFlows] = useState<Flow[]>([]);
  const [stats, setStats] = useState<FlowStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(false);

  // Advanced filters
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [protocol, setProtocol] = useState<Protocol>('ALL');
  const [portFilter, setPortFilter] = useState('');
  const [timeRange, setTimeRange] = useState<TimeRange>('ALL');
  const [httpMethod, setHttpMethod] = useState<HttpMethod>('ALL');
  const [httpStatusFilter, setHttpStatusFilter] = useState('');

  // L7 column toggle (null = auto-detect)
  const [showL7Override, setShowL7Override] = useState<boolean | null>(null);

  const fetchData = useCallback(async () => {
    setError(null);
    try {
      const params: Record<string, string | number> = { limit: pagination.limit, offset: pagination.offset };
      if (namespace) params.namespace = namespace;
      if (verdict !== 'ALL') params.verdict = verdict;
      const [flowRes, statsRes] = await Promise.all([apiFetchFlows(params), apiFetchFlowStats().catch(() => null)]);
      setFlows(flowRes.data.flows);
      pagination.setTotal(flowRes.data.total);
      if (statsRes) setStats(statsRes.data);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch flows');
    }
  }, [namespace, verdict, pagination.offset, pagination.limit, pagination.setTotal]);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 10000, autoRefreshOn);

  // Detect if any flows carry L7 HTTP data
  const hasL7Data = useMemo(() => flows.some((f) => f.http_method != null || f.http_url != null || f.http_code != null), [flows]);

  // L7 columns shown if user toggled on, or auto-detected
  const showL7 = showL7Override !== null ? showL7Override : hasL7Data;

  // Count active advanced filters
  const activeFilterCount = useMemo(() => {
    let count = 0;
    if (protocol !== 'ALL') count++;
    if (portFilter.trim()) count++;
    if (timeRange !== 'ALL') count++;
    if (httpMethod !== 'ALL') count++;
    if (httpStatusFilter.trim()) count++;
    return count;
  }, [protocol, portFilter, timeRange, httpMethod, httpStatusFilter]);

  const filtered = useMemo(() => {
    const portMatcher = parsePortFilter(portFilter);
    const statusMatcher = parseHttpStatusFilter(httpStatusFilter);
    const now = Date.now();
    const timeMs = TIME_RANGE_MS[timeRange];

    return flows.filter((f) => {
      // Search text filter
      if (searchText) {
        const t = searchText.toLowerCase();
        const matches =
          f.source.pod.toLowerCase().includes(t) ||
          f.source.namespace.toLowerCase().includes(t) ||
          f.destination.pod.toLowerCase().includes(t) ||
          f.destination.namespace.toLowerCase().includes(t) ||
          f.protocol.toLowerCase().includes(t) ||
          String(f.port).includes(t) ||
          (f.http_method?.toLowerCase().includes(t) ?? false) ||
          (f.http_url?.toLowerCase().includes(t) ?? false) ||
          (f.http_code != null && String(f.http_code).includes(t));
        if (!matches) return false;
      }
      // Protocol filter
      if (protocol !== 'ALL' && f.protocol.toUpperCase() !== protocol) return false;
      // Port filter
      if (portMatcher && !portMatcher(f.port)) return false;
      // Time range filter
      if (timeMs > 0) {
        try {
          const ts = new Date(f.timestamp).getTime();
          if (now - ts > timeMs) return false;
        } catch {
          // keep the flow if we cannot parse its timestamp
        }
      }
      // L7 HTTP method filter
      if (httpMethod !== 'ALL' && f.http_method != null) {
        if (String(f.http_method).toUpperCase() !== httpMethod) return false;
      }
      // L7 HTTP status filter
      if (statusMatcher && f.http_code != null) {
        if (!statusMatcher(f.http_code)) return false;
      }
      return true;
    });
  }, [flows, searchText, protocol, portFilter, timeRange, httpMethod, httpStatusFilter]);

  const fmtTime = (ts: string) => {
    try { const d = new Date(ts); return `${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}:${String(d.getSeconds()).padStart(2,'0')}`; } catch { return ts; }
  };

  // Donut chart data
  const donutData = useMemo(() => {
    const data: { name: string; value: number }[] = [];
    const fwd = stats?.forwarded ?? 0;
    const drp = stats?.dropped ?? 0;
    const rdr = stats?.redirected ?? 0;
    if (fwd > 0) data.push({ name: 'Forwarded', value: fwd });
    if (drp > 0) data.push({ name: 'Dropped', value: drp });
    if (rdr > 0) data.push({ name: 'Redirected', value: rdr });
    // If all zero, show a placeholder ring
    if (data.length === 0) data.push({ name: 'Forwarded', value: 1 });
    return data;
  }, [stats]);

  const totalFlows = stats?.total_flows ?? 0;
  const colSpan = showL7 ? 9 : 6;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
            <Gauge className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">Flow Monitoring</h1>
            <p className="text-sm text-slate-400">Real-time network flow analysis</p>
          </div>
        </div>
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
          autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={10} />
      </div>

      {error && (
        <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm flex items-center justify-between">
          {error}
          <button onClick={() => setError(null)} className="text-red-400 hover:text-red-300">&times;</button>
        </div>
      )}

      {/* Verdict breakdown donut chart + stat badges */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 mb-4">
        <div className="flex items-center gap-6">
          {/* Donut chart */}
          <div className="relative w-28 h-28 flex-shrink-0">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={donutData}
                  cx="50%"
                  cy="50%"
                  innerRadius={30}
                  outerRadius={48}
                  dataKey="value"
                  strokeWidth={0}
                >
                  {donutData.map((entry) => (
                    <Cell key={entry.name} fill={DONUT_COLORS[entry.name] ?? '#64748b'} />
                  ))}
                </Pie>
              </PieChart>
            </ResponsiveContainer>
            {/* Center label */}
            <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
              <span className="text-lg font-bold text-white leading-none">{totalFlows}</span>
              <span className="text-[10px] text-slate-400">flows</span>
            </div>
          </div>

          {/* Stat badges */}
          <div className="flex flex-wrap items-center gap-3">
            <StatBadge icon={<ArrowUpRight className="w-3.5 h-3.5 text-green-400" />} label="Forwarded" value={stats?.forwarded ?? 0} color="text-green-400" />
            <StatBadge icon={<ArrowDownRight className="w-3.5 h-3.5 text-red-400" />} label="Dropped" value={stats?.dropped ?? 0} color="text-red-400" />
            {(stats?.redirected ?? 0) > 0 && (
              <StatBadge icon={<Shuffle className="w-3.5 h-3.5 text-blue-400" />} label="Redirected" value={stats?.redirected ?? 0} color="text-blue-400" />
            )}
            <StatBadge icon={<Gauge className="w-3.5 h-3.5 text-slate-400" />} label="Req/s" value={stats?.requests_per_second ?? 0} color="text-white" />
          </div>
        </div>
      </div>

      {/* Filter bar */}
      <div className="mb-4 rounded-lg border border-slate-700/50 bg-slate-800/50">
        <div className="flex flex-wrap items-center gap-3 p-3">
          <input
            type="text"
            value={namespace}
            onChange={(e) => { setNamespace(e.target.value); pagination.resetPage(); }}
            placeholder="Namespace"
            className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 w-40"
          />
          <select
            value={verdict}
            onChange={(e) => { setVerdict(e.target.value as Verdict); pagination.resetPage(); }}
            className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="ALL">All Verdicts</option>
            <option value="FORWARDED">Forwarded</option>
            <option value="DROPPED">Dropped</option>
            <option value="AUDIT">Audit</option>
            <option value="REDIRECTED">Redirected</option>
          </select>
          <div className="relative flex-1 min-w-[200px]">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
            <input
              type="text"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              placeholder="Search pod, protocol, port, URL..."
              className="w-full pl-9 pr-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <button
            onClick={() => setShowAdvanced((v) => !v)}
            className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-sm font-medium transition-colors ${
              showAdvanced || activeFilterCount > 0
                ? 'bg-blue-500/15 border-blue-500/40 text-blue-400'
                : 'bg-slate-900/50 border-slate-700/50 text-slate-400 hover:text-white'
            }`}
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
            Filters
            {activeFilterCount > 0 && (
              <span className="ml-1 px-1.5 py-0.5 rounded-full bg-blue-500 text-white text-xs leading-none">
                {activeFilterCount}
              </span>
            )}
            {showAdvanced ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
          </button>
        </div>

        {/* Advanced filters (collapsible) */}
        {showAdvanced && (
          <div className="border-t border-slate-700/50 px-3 py-3 flex flex-wrap items-center gap-3">
            <select
              value={protocol}
              onChange={(e) => setProtocol(e.target.value as Protocol)}
              className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="ALL">All Protocols</option>
              <option value="TCP">TCP</option>
              <option value="UDP">UDP</option>
              <option value="ICMP">ICMP</option>
              <option value="SCTP">SCTP</option>
            </select>
            <input
              type="text"
              value={portFilter}
              onChange={(e) => setPortFilter(e.target.value)}
              placeholder="Port or range"
              className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 w-32"
            />
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value as TimeRange)}
              className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {(Object.keys(TIME_RANGE_LABELS) as TimeRange[]).map((k) => (
                <option key={k} value={k}>{TIME_RANGE_LABELS[k]}</option>
              ))}
            </select>

            {/* L7 HTTP filters -- only visible when flows carry L7 data */}
            {hasL7Data && (
              <>
                <div className="w-px h-6 bg-slate-700/60" />
                <span className="text-xs text-slate-500 uppercase tracking-wide">L7</span>
                <select
                  value={httpMethod}
                  onChange={(e) => setHttpMethod(e.target.value as HttpMethod)}
                  className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="ALL">All Methods</option>
                  <option value="GET">GET</option>
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                  <option value="DELETE">DELETE</option>
                  <option value="PATCH">PATCH</option>
                </select>
                <input
                  type="text"
                  value={httpStatusFilter}
                  onChange={(e) => setHttpStatusFilter(e.target.value)}
                  placeholder="Status (e.g. 500, 4xx)"
                  className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 w-40"
                />
              </>
            )}
          </div>
        )}
      </div>

      {/* Table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <Gauge className="w-4 h-4 text-white" />
            </div>
            <h2 className="text-lg font-semibold text-white">Network Flows</h2>
          </div>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={showL7}
                onChange={(e) => setShowL7Override(e.target.checked)}
                className="rounded border-slate-600 bg-slate-900/50 text-blue-500 focus:ring-blue-500 focus:ring-offset-0 w-3.5 h-3.5"
              />
              <span className="text-xs text-slate-400">Show L7 Details</span>
            </label>
            <span className="text-xs font-medium text-slate-400 bg-slate-700/50 px-2.5 py-1 rounded-full">
              {filtered.length} of {pagination.total}
            </span>
          </div>
        </div>
        {loading && (
          <div className="flex justify-center p-3">
            <Loader2 className="w-5 h-5 animate-spin text-blue-400" />
          </div>
        )}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/50 bg-slate-900/50">
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Time</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Source</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Destination</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Protocol</th>
                {showL7 && (
                  <>
                    <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Method</th>
                    <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">URL</th>
                    <th className="text-center px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Status</th>
                  </>
                )}
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Port</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Verdict</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 && !loading ? (
                <tr>
                  <td colSpan={colSpan} className="px-4 py-12 text-center text-slate-400">
                    No flows found. Connect to Hubble to see network flows.
                  </td>
                </tr>
              ) : (
                filtered.map((f) => (
                  <tr key={f.id} className="border-b border-slate-700/30 table-row-hover">
                    <td className="px-4 py-2.5 whitespace-nowrap text-slate-400">{fmtTime(f.timestamp)}</td>
                    <td className="px-4 py-2.5">
                      <div className="font-medium text-white">{f.source.namespace}/{f.source.pod}</div>
                      <div className="text-xs text-slate-400">{f.source.ip}</div>
                    </td>
                    <td className="px-4 py-2.5">
                      <div className="font-medium text-white">{f.destination.namespace}/{f.destination.pod}</div>
                      <div className="text-xs text-slate-400">{f.destination.ip}</div>
                    </td>
                    <td className="px-4 py-2.5">
                      <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{f.protocol}</span>
                    </td>
                    {showL7 && (
                      <>
                        <td className="px-4 py-2.5 whitespace-nowrap">
                          {f.http_method ? (
                            <span className="px-2 py-0.5 rounded bg-slate-700/50 text-xs font-mono font-medium text-slate-200">{f.http_method}</span>
                          ) : (
                            <span className="text-slate-600">&mdash;</span>
                          )}
                        </td>
                        <td className="px-4 py-2.5 max-w-[280px]">
                          {f.http_url ? (
                            <span className="text-xs text-slate-300 font-mono" title={f.http_url}>
                              {truncateUrl(f.http_url, 40)}
                            </span>
                          ) : (
                            <span className="text-slate-600">&mdash;</span>
                          )}
                        </td>
                        <td className="px-4 py-2.5 text-center">
                          {f.http_code != null ? (
                            <span className={`inline-block px-2 py-0.5 rounded border text-xs font-mono font-medium ${httpStatusColor(f.http_code)} ${httpStatusBg(f.http_code)}`}>
                              {f.http_code}
                            </span>
                          ) : (
                            <span className="text-slate-600">&mdash;</span>
                          )}
                        </td>
                      </>
                    )}
                    <td className="px-4 py-2.5 text-right font-mono">{f.port}</td>
                    <td className="px-4 py-2.5">
                      <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border ${VERDICT_BADGE[f.verdict] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>
                        {VERDICT_ICON[f.verdict]}
                        {f.verdict}
                      </span>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-slate-700/50 text-sm text-slate-400">
          <span>Showing {pagination.pageRange.start}&ndash;{pagination.pageRange.end} of {pagination.total} flows</span>
          <div className="flex items-center gap-2">
            <span className="text-xs">Page {pagination.page + 1} of {pagination.totalPages || 1}</span>
            <button
              disabled={!pagination.hasPrevPage}
              onClick={pagination.prevPage}
              className="px-3 py-1 rounded border border-slate-700/50 hover:bg-slate-700/30 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Previous
            </button>
            <button
              disabled={!pagination.hasNextPage}
              onClick={pagination.nextPage}
              className="px-3 py-1 rounded border border-slate-700/50 hover:bg-slate-700/30 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Next
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

function StatBadge({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: number; color: string }) {
  return (
    <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 bg-slate-900/40">
      {icon}
      <div className="flex flex-col">
        <span className="text-[10px] text-slate-500 leading-none">{label}</span>
        <span className={`text-sm font-bold ${color} leading-tight`}>{value}</span>
      </div>
    </div>
  );
}

export default Flows;
