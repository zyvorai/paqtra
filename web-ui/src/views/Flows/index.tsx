import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Search,
  RefreshCw,
  ArrowRight,
  Ban,
  Eye,
  Gauge,
  ArrowUpRight,
  ArrowDownRight,
  Loader2,
} from 'lucide-react';
import { fetchFlows as apiFetchFlows, fetchFlowStats as apiFetchFlowStats, Flow, FlowStats } from '../../services/api';
import { isAxiosError } from 'axios';
import { format, parseISO } from 'date-fns';
import { usePageTitle } from '../../hooks/usePageTitle';

type Verdict = 'ALL' | 'FORWARDED' | 'DROPPED' | 'AUDIT';

const VERDICT_BADGE: Record<string, string> = {
  FORWARDED: 'bg-green-500/15 text-green-400 border-green-500/30',
  DROPPED: 'bg-red-500/15 text-red-400 border-red-500/30',
  AUDIT: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
};

const VERDICT_ICON: Record<string, React.ReactNode> = {
  FORWARDED: <ArrowRight className="w-3 h-3" />,
  DROPPED: <Ban className="w-3 h-3" />,
  AUDIT: <Eye className="w-3 h-3" />,
};

const Flows: React.FC = () => {
  usePageTitle('Flows');
  const [namespace, setNamespace] = useState('');
  const [verdict, setVerdict] = useState<Verdict>('ALL');
  const [searchText, setSearchText] = useState('');
  const [page, setPage] = useState(0);
  const [rowsPerPage] = useState(25);
  const [flows, setFlows] = useState<Flow[]>([]);
  const [total, setTotal] = useState(0);
  const [stats, setStats] = useState<FlowStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchFlows = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params: Record<string, string | number> = { limit: rowsPerPage, offset: page * rowsPerPage };
      if (namespace) params.namespace = namespace;
      if (verdict !== 'ALL') params.verdict = verdict;
      const res = await apiFetchFlows(params);
      setFlows(res.data.flows);
      setTotal(res.data.total);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch flows');
    } finally {
      setLoading(false);
    }
  }, [namespace, verdict, page, rowsPerPage]);

  const fetchStats = useCallback(async () => {
    try {
      const res = await apiFetchFlowStats();
      setStats(res.data);
    } catch { /* non-critical */ }
  }, []);

  useEffect(() => { fetchFlows(); fetchStats(); }, [fetchFlows, fetchStats]);

  useEffect(() => {
    if (autoRefresh) {
      intervalRef.current = setInterval(() => { fetchFlows(); fetchStats(); }, 10000);
    }
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, [autoRefresh, fetchFlows, fetchStats]);

  const filtered = searchText
    ? flows.filter((f) => {
        const t = searchText.toLowerCase();
        return (
          f.source.pod.toLowerCase().includes(t) ||
          f.source.namespace.toLowerCase().includes(t) ||
          f.destination.pod.toLowerCase().includes(t) ||
          f.destination.namespace.toLowerCase().includes(t) ||
          f.protocol.toLowerCase().includes(t) ||
          String(f.port).includes(t)
        );
      })
    : flows;

  const fmtTime = (ts: string) => {
    try { return format(parseISO(ts), 'HH:mm:ss'); } catch { return ts; }
  };

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
        <button onClick={() => { fetchFlows(); fetchStats(); }} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          Refresh
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm flex items-center justify-between">
          {error}
          <button onClick={() => setError(null)} className="text-red-400 hover:text-red-300">&times;</button>
        </div>
      )}

      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
        <MiniStat icon={<Gauge className="w-4 h-4" />} label="Total Flows" value={stats?.total_flows ?? 0} />
        <MiniStat icon={<ArrowUpRight className="w-4 h-4 text-green-400" />} label="Forwarded" value={stats?.forwarded ?? 0} valueColor="text-green-400" />
        <MiniStat icon={<ArrowDownRight className="w-4 h-4 text-red-400" />} label="Dropped" value={stats?.dropped ?? 0} valueColor="text-red-400" />
        <MiniStat icon={<Gauge className="w-4 h-4 text-blue-400" />} label="Req/s" value={stats?.requests_per_second ?? 0} />
      </div>

      {/* Filter bar */}
      <div className="flex flex-wrap items-center gap-3 mb-4 p-3 rounded-lg border border-slate-700/50 bg-slate-800/50">
        <input
          type="text"
          value={namespace}
          onChange={(e) => { setNamespace(e.target.value); setPage(0); }}
          placeholder="Namespace"
          className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500 w-40"
        />
        <select
          value={verdict}
          onChange={(e) => { setVerdict(e.target.value as Verdict); setPage(0); }}
          className="px-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="ALL">All Verdicts</option>
          <option value="FORWARDED">Forwarded</option>
          <option value="DROPPED">Dropped</option>
          <option value="AUDIT">Audit</option>
        </select>
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input
            type="text"
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            placeholder="Search pod, protocol, port..."
            className="w-full pl-9 pr-3 py-1.5 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
        <label className="flex items-center gap-2 text-sm text-slate-400 cursor-pointer">
          <input type="checkbox" checked={autoRefresh} onChange={(e) => setAutoRefresh(e.target.checked)} className="rounded" />
          Auto-refresh
        </label>
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
          <span className="text-xs font-medium text-slate-400 bg-slate-700/50 px-2.5 py-1 rounded-full">
            {filtered.length} of {total}
          </span>
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
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Port</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Verdict</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 && !loading ? (
                <tr>
                  <td colSpan={6} className="px-4 py-12 text-center text-slate-400">
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
          <span>Showing {filtered.length} of {total} flows</span>
          <div className="flex gap-2">
            <button
              disabled={page === 0}
              onClick={() => setPage((p) => p - 1)}
              className="px-3 py-1 rounded border border-slate-700/50 hover:bg-slate-700/30 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Previous
            </button>
            <button
              disabled={(page + 1) * rowsPerPage >= total}
              onClick={() => setPage((p) => p + 1)}
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

function MiniStat({ icon, label, value, valueColor }: { icon: React.ReactNode; label: string; value: number; valueColor?: string }) {
  return (
    <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
      <div className="flex items-center gap-2 mb-1">
        <span className="text-slate-400">{icon}</span>
        <span className="text-xs text-slate-400">{label}</span>
      </div>
      <div className={`text-xl font-bold ${valueColor ?? 'text-white'}`}>{value}</div>
    </div>
  );
}

export default Flows;
