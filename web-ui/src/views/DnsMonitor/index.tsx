import React, { useState, useCallback } from 'react';
import { Globe2, Loader2, Search } from 'lucide-react';
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

      {/* Stats */}
      {stats && (
        <div className="grid grid-cols-2 lg:grid-cols-5 gap-3 mb-6">
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Total Queries</div>
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
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Query</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Type</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Response</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Latency</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 && !loading ? (
                <tr><td colSpan={6} className="px-4 py-12 text-center text-slate-400">No DNS queries found.</td></tr>
              ) : filtered.map((q) => (
                <tr key={q.id} className="border-b border-slate-700/30 table-row-hover">
                  <td className="px-4 py-2.5 whitespace-nowrap text-slate-400">{formatRelativeTime(q.timestamp)}</td>
                  <td className="px-4 py-2.5">
                    <div className="font-medium text-white">{q.source_pod}</div>
                    <div className="text-xs text-slate-500">{q.namespace}</div>
                  </td>
                  <td className="px-4 py-2.5 font-mono text-xs text-white">{q.query_name}</td>
                  <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{q.query_type}</span></td>
                  <td className="px-4 py-2.5">
                    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs border ${RCODE_BADGE[q.response_code] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>
                      {q.response_code}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-right font-mono text-slate-400">{q.latency_ms.toFixed(1)} ms</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default DnsMonitor;
