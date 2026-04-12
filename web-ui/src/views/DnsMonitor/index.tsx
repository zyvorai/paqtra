import React, { useState, useCallback, useMemo } from 'react';
import { Globe2, Loader2, Search, ArrowRight, Ban, AlertTriangle } from 'lucide-react';
import { fetchDnsQueries, fetchDnsStats, DnsQuery, DnsStats } from '../../services/api';
import { usePageTitle } from '../../hooks/usePageTitle';
import { formatRelativeTime } from '../../utils/formatters';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const RCODE_BADGE: Record<string, string> = {
  NOERROR: 'bg-green-500/15 text-green-400 border-green-500/30',
  NXDOMAIN: 'bg-red-500/15 text-red-400 border-red-500/30',
  SERVFAIL: 'bg-red-500/15 text-red-400 border-red-500/30',
  REFUSED: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
};

/** Map response codes to a verdict for color-coding rows */
function dnsVerdict(rcode: string): 'FORWARDED' | 'DROPPED' {
  return rcode === 'NOERROR' ? 'FORWARDED' : 'DROPPED';
}

const VERDICT_ROW_CLASS: Record<string, string> = {
  FORWARDED: 'border-l-2 border-l-green-500/40',
  DROPPED: 'border-l-2 border-l-red-500/40',
};

const DnsMonitor: React.FC = () => {
  usePageTitle('DNS Monitor');
  const [queries, setQueries] = useState<DnsQuery[]>([]);
  const [stats, setStats] = useState<DnsStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchText, setSearchText] = useState('');
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try {
      const [qRes, sRes] = await Promise.all([fetchDnsQueries(), fetchDnsStats()]);
      setQueries(qRes.data.queries);
      setStats(sRes.data);
    } catch {
      setError('Failed to fetch DNS data');
    }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const filtered = searchText
    ? queries.filter((q) => q.query_name.toLowerCase().includes(searchText.toLowerCase()) || q.source_pod.toLowerCase().includes(searchText.toLowerCase()))
    : queries;

  /** Derived DNS-specific stats: forwarded, dropped, drop rate */
  const dnsFlowStats = useMemo(() => {
    const total = queries.length;
    const forwarded = queries.filter((q) => dnsVerdict(q.response_code) === 'FORWARDED').length;
    const dropped = total - forwarded;
    const dropRate = total > 0 ? ((dropped / total) * 100).toFixed(1) : '0.0';
    return { total, forwarded, dropped, dropRate };
  }, [queries]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
            <Globe2 className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">DNS Monitor</h1>
            <p className="text-sm text-slate-400">Track DNS queries and resolution across the cluster</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <ExportButton data={filtered as Record<string, unknown>[]} filename="dns-queries" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* DNS-specific stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-2 mb-1">
            <Globe2 className="w-4 h-4 text-blue-400" />
            <span className="text-xs text-slate-400">Total DNS Queries</span>
          </div>
          <div className="text-2xl font-bold text-white">{dnsFlowStats.total}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-2 mb-1">
            <ArrowRight className="w-4 h-4 text-green-400" />
            <span className="text-xs text-slate-400">Forwarded (Resolved)</span>
          </div>
          <div className="text-2xl font-bold text-green-400">{dnsFlowStats.forwarded}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-2 mb-1">
            <Ban className="w-4 h-4 text-red-400" />
            <span className="text-xs text-slate-400">Dropped (Failed)</span>
          </div>
          <div className="text-2xl font-bold text-red-400">{dnsFlowStats.dropped}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-2 mb-1">
            <AlertTriangle className="w-4 h-4 text-yellow-400" />
            <span className="text-xs text-slate-400">Drop Rate</span>
          </div>
          <div className={`text-2xl font-bold ${Number(dnsFlowStats.dropRate) > 5 ? 'text-red-400' : Number(dnsFlowStats.dropRate) > 0 ? 'text-yellow-400' : 'text-green-400'}`}>
            {dnsFlowStats.dropRate}%
          </div>
        </div>
      </div>

      {/* Detailed API stats */}
      {stats && (
        <div className="grid grid-cols-2 lg:grid-cols-5 gap-3 mb-6">
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Total (API)</div>
            <div className="text-2xl font-bold text-white">{stats.total_queries}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Successful</div>
            <div className="text-2xl font-bold text-green-400">{stats.successful}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">NXDOMAIN</div>
            <div className="text-2xl font-bold text-red-400">{stats.nxdomain}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">SERVFAIL</div>
            <div className="text-2xl font-bold text-yellow-400">{stats.servfail}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Avg Latency</div>
            <div className="text-2xl font-bold text-white">{stats.avg_latency_ms.toFixed(1)} ms</div>
          </div>
        </div>
      )}

      {/* Search filter */}
      <div className="flex items-center gap-3 mb-4 p-3 rounded-xl border border-slate-700/50 bg-slate-800/50">
        <Search className="w-4 h-4 text-slate-400" />
        <input
          type="text"
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          placeholder="Search by domain or pod..."
          className="flex-1 bg-transparent text-sm text-white placeholder-slate-500 focus:outline-none"
        />
      </div>

      {/* Table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <Globe2 className="w-4 h-4 text-white" />
            </div>
            <h2 className="text-lg font-semibold text-white">DNS Queries</h2>
          </div>
          <span className="text-xs font-medium text-slate-400 bg-slate-700/50 px-2.5 py-1 rounded-full">
            {filtered.length} queries
          </span>
        </div>
        {loading && <div className="flex justify-center p-3"><Loader2 className="w-5 h-5 animate-spin text-blue-400" /></div>}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/50 bg-slate-900/50">
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Time</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Source Pod</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Query Name / Destination</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Protocol</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Verdict</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Response</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Latency</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 && !loading ? (
                <tr><td colSpan={7} className="px-4 py-12 text-center text-slate-400">No DNS queries found.</td></tr>
              ) : filtered.map((q) => {
                const verdict = dnsVerdict(q.response_code);
                return (
                  <tr key={q.id} className={`border-b border-slate-700/30 table-row-hover ${VERDICT_ROW_CLASS[verdict]}`}>
                    <td className="px-4 py-2.5 whitespace-nowrap text-slate-400">{formatRelativeTime(q.timestamp)}</td>
                    <td className="px-4 py-2.5">
                      <div className="font-medium text-white">{q.source_pod}</div>
                      <div className="text-xs text-slate-500">{q.namespace}</div>
                    </td>
                    <td className="px-4 py-2.5 font-mono text-xs text-white">{q.query_name}</td>
                    <td className="px-4 py-2.5">
                      <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">DNS ({q.query_type})</span>
                    </td>
                    <td className="px-4 py-2.5">
                      <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border ${
                        verdict === 'FORWARDED'
                          ? 'bg-green-500/15 text-green-400 border-green-500/30'
                          : 'bg-red-500/15 text-red-400 border-red-500/30'
                      }`}>
                        {verdict === 'FORWARDED' ? <ArrowRight className="w-3 h-3" /> : <Ban className="w-3 h-3" />}
                        {verdict}
                      </span>
                    </td>
                    <td className="px-4 py-2.5">
                      <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs border ${RCODE_BADGE[q.response_code] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>
                        {q.response_code}
                      </span>
                    </td>
                    <td className="px-4 py-2.5 text-right font-mono text-slate-400">{q.latency_ms.toFixed(1)} ms</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default DnsMonitor;
