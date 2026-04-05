import React, { useState, useCallback } from 'react';
import { History, Loader2, RotateCcw, Search } from 'lucide-react';
import { fetchChangeLog, rollbackChange, ChangeEntry } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoDismiss } from '../../hooks/useAutoDismiss';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const TYPE_BADGE: Record<string, string> = { create: 'bg-green-500/15 text-green-400 border-green-500/30', update: 'bg-blue-500/15 text-blue-400 border-blue-500/30', delete: 'bg-red-500/15 text-red-400 border-red-500/30' };

const ChangeLog: React.FC = () => {
  usePageTitle('Change Log');
  const [changes, setChanges] = useState<ChangeEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useAutoDismiss<string | null>(null);
  const [search, setSearch] = useState('');
  const [rolling, setRolling] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setChanges((await fetchChangeLog()).data.changes ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const handleRollback = async (id: string) => {
    setRolling(id); setError(null);
    try { await rollbackChange(id); setSuccess('Rollback applied'); manualRefresh(); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setRolling(null); }
  };

  const filtered = search ? changes.filter((c) => c.resource.toLowerCase().includes(search.toLowerCase()) || c.author.toLowerCase().includes(search.toLowerCase()) || c.diff_summary.toLowerCase().includes(search.toLowerCase())) : changes;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 flex items-center justify-center shadow-lg shadow-slate-500/20"><History className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Change Log</h1></div>
          <p className="text-sm text-slate-400 mt-1">Configuration change history with rollback</p>
        </div>
        <div className="flex items-center gap-3">
          <ExportButton data={filtered as unknown as Record<string, unknown>[]} filename="change-log" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {success && <div className="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-sm">{success}</div>}

      <div className="relative mb-4"><Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" /><input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search changes..." className="w-full pl-9 pr-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500" /></div>

      {loading && changes.length === 0 && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {!loading && filtered.length === 0 && !error && (
        <div className="text-center py-12 text-slate-400">No changes recorded.</div>
      )}

      <div className="relative pl-6">
        <div className="absolute left-2.5 top-0 bottom-0 w-px bg-slate-700" />
        {filtered.map((c) => (
          <div key={c.id} className="relative mb-4">
            <div className={`absolute -left-[14px] w-3 h-3 rounded-full border-2 border-slate-800 ${c.type === 'create' ? 'bg-green-400' : c.type === 'delete' ? 'bg-red-400' : 'bg-blue-400'}`} />
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 ml-2">
              <div className="flex items-center gap-2 mb-2">
                <span className={`px-2 py-0.5 rounded-full text-xs border ${TYPE_BADGE[c.type] ?? ''}`}>{c.type}</span>
                <span className="font-mono text-sm text-white">{c.resource}</span>
                {c.namespace && <span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-xs text-slate-400">{c.namespace}</span>}
                <span className="ml-auto text-xs text-slate-400">{new Date(c.timestamp).toLocaleString()}</span>
              </div>
              <div className="text-sm text-slate-400 mb-2">{c.diff_summary}</div>
              <div className="flex items-center justify-between text-xs text-slate-400">
                <span>By: <span className="text-white">{c.author}</span></span>
                {c.rollback_available && (
                  <button onClick={() => handleRollback(c.id)} disabled={rolling === c.id} className="flex items-center gap-1 px-2 py-1 rounded border border-slate-700/50 hover:bg-slate-700/30 disabled:opacity-50 transition-colors text-white">
                    {rolling === c.id ? <Loader2 className="w-3 h-3 animate-spin" /> : <RotateCcw className="w-3 h-3" />} Rollback
                  </button>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ChangeLog;
