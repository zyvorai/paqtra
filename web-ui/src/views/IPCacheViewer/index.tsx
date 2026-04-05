import React, { useState, useCallback, useMemo } from 'react';
import { Globe } from 'lucide-react';
import { fetchEbpfIpcache } from '../../services/api';
import { isAxiosError } from 'axios';
import { formatCount } from '../../utils/formatters';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

interface IPCacheEntry {
  ip: string;
  cidr: string;
  identity: number;
  [key: string]: unknown;
}

const IDENTITY_COLORS = [
  'bg-blue-500/20 text-blue-400 border-blue-500/30',
  'bg-green-500/20 text-green-400 border-green-500/30',
  'bg-purple-500/20 text-purple-400 border-purple-500/30',
  'bg-orange-500/20 text-orange-400 border-orange-500/30',
  'bg-cyan-500/20 text-cyan-400 border-cyan-500/30',
  'bg-pink-500/20 text-pink-400 border-pink-500/30',
  'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
];

function identityColor(id: number): string {
  return IDENTITY_COLORS[id % IDENTITY_COLORS.length];
}

const IPCacheViewer: React.FC = () => {
  usePageTitle('IP Cache');
  const [entries, setEntries] = useState<IPCacheEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);
  const [search, setSearch] = useState('');

  const loadData = useCallback(async () => {
    setError(null);
    try {
      const res = await fetchEbpfIpcache();
      setEntries((res.data.entries ?? []) as IPCacheEntry[]);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load IP cache data');
    }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(loadData, 30000, autoRefreshOn);

  const filtered = useMemo(() => {
    if (!search) return entries;
    const q = search.toLowerCase();
    return entries.filter((e) => (e.ip ?? e.cidr ?? '').toLowerCase().includes(q));
  }, [entries, search]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-emerald-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <Globe className="w-5 h-5 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-white">IP Cache</h1>
          </div>
          <p className="text-sm text-slate-400 mt-1">IP to identity mapping from kernel</p>
        </div>
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-2 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Entries</div>
          <div className="text-2xl font-bold text-blue-400">{formatCount(entries.length)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Unique Identities</div>
          <div className="text-2xl font-bold text-green-400">{formatCount(new Set(entries.map((e) => e.identity)).size)}</div>
        </div>
      </div>

      <div className="mb-4">
        <input
          type="text"
          placeholder="Search by IP or CIDR..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full max-w-sm px-4 py-2 rounded-lg bg-slate-800 border border-slate-700 text-white text-sm placeholder-slate-500 focus:outline-none focus:border-emerald-500"
        />
      </div>

      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        {filtered.length > 0 && (
          <div className="px-4 py-3 border-b border-slate-700/50 flex justify-end">
            <ExportButton data={filtered as unknown as Record<string, unknown>[]} filename="ipcache-entries" />
          </div>
        )}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-slate-900/50">
              <tr className="text-left text-slate-400">
                <th className="px-4 py-3 font-medium">IP / CIDR</th>
                <th className="px-4 py-3 font-medium">Identity</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/50">
              {filtered.map((e, i) => (
                <tr key={i} className="table-row-hover">
                  <td className="px-4 py-3 text-white font-mono text-xs">{e.ip || e.cidr}</td>
                  <td className="px-4 py-3">
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${identityColor(e.identity)}`}>
                      {e.identity}
                    </span>
                  </td>
                </tr>
              ))}
              {filtered.length === 0 && !loading && (
                <tr><td colSpan={2} className="px-4 py-8 text-center text-slate-400">No IP cache entries found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default IPCacheViewer;
