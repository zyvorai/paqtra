import React, { useState, useCallback, useMemo } from 'react';
import { ScrollText, Loader2, Search } from 'lucide-react';
import { fetchAuditLog, AuditEntry } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { usePagination } from '../../hooks/usePagination';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const ACTION_BADGE: Record<string, string> = {
  'policy.create': 'bg-green-500/15 text-green-400 border-green-500/30',
  'policy.delete': 'bg-red-500/15 text-red-400 border-red-500/30',
  'policy.simulate': 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  'anomaly.remediate': 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  'chaos.run': 'bg-purple-500/15 text-purple-400 border-purple-500/30',
  'compliance.audit': 'bg-cyan-500/15 text-cyan-400 border-cyan-500/30',
};

const OUTCOME_BADGE: Record<string, string> = {
  success: 'bg-green-500/15 text-green-400 border-green-500/30',
  failure: 'bg-red-500/15 text-red-400 border-red-500/30',
};

function formatTime(ts: string): string {
  try { return new Date(ts).toLocaleString(); } catch { return ts; }
}

const AuditLog: React.FC = () => {
  usePageTitle('Audit Log');
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);
  const pagination = usePagination({ initialLimit: 25 });

  const fetchData = useCallback(async () => {
    setError(null);
    try { setEntries((await fetchAuditLog()).data.entries ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const filtered = useMemo(() => search ? entries.filter((e) => e.action.includes(search.toLowerCase()) || e.actor.toLowerCase().includes(search.toLowerCase()) || e.resource.toLowerCase().includes(search.toLowerCase()) || e.details.toLowerCase().includes(search.toLowerCase())) : entries, [entries, search]);

  // Keep pagination total in sync with filtered count and reset page on search change
  useMemo(() => { pagination.setTotal(filtered.length); pagination.resetPage(); }, [filtered.length]); // eslint-disable-line react-hooks/exhaustive-deps

  const paginatedEntries = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.limit),
    [filtered, pagination.offset, pagination.limit],
  );

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-indigo-500 to-indigo-700 flex items-center justify-center shadow-lg shadow-indigo-500/20"><ScrollText className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Audit Log</h1></div>
          <p className="text-sm text-slate-400 mt-1">Security audit trail of all actions</p>
        </div>
        <div className="flex items-center gap-2">
          <ExportButton data={filtered as Record<string, unknown>[]} filename="audit-log" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="relative mb-4">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
        <input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search actions, actors, resources..."
          className="w-full pl-9 pr-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500" />
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="space-y-2">
        {paginatedEntries.map((e) => (
          <div key={e.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-center gap-3 mb-2">
              <span className={`px-2 py-0.5 rounded-full text-xs border ${ACTION_BADGE[e.action] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>{e.action}</span>
              <span className={`px-2 py-0.5 rounded-full text-xs border ${OUTCOME_BADGE[e.outcome] ?? ''}`}>{e.outcome}</span>
              <span className="ml-auto text-xs text-slate-400">{formatTime(e.timestamp)}</span>
            </div>
            <div className="text-sm text-white mb-1">{e.details}</div>
            <div className="flex items-center gap-4 text-xs text-slate-400">
              <span>Actor: <span className="text-white">{e.actor}</span></span>
              <span>Resource: <span className="text-white font-mono">{e.resource}</span></span>
              {e.namespace !== '-' && <span>Namespace: <span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-white">{e.namespace}</span></span>}
            </div>
          </div>
        ))}
        {!loading && paginatedEntries.length === 0 && <div className="text-center py-12 text-slate-400">No audit entries found</div>}
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between mt-4 px-4 py-3 rounded-xl border border-slate-700/50 bg-slate-800/50 text-sm text-slate-400">
        <span>Showing {pagination.pageRange.start}–{pagination.pageRange.end} of {filtered.length} entries</span>
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
  );
};

export default AuditLog;
