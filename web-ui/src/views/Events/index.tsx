import React, { useState, useCallback } from 'react';
import { Bell, Loader2, AlertTriangle, Info, CheckCircle } from 'lucide-react';
import { fetchEvents, K8sEvent } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const TYPE_BADGE: Record<string, string> = {
  Normal: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  Warning: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  Error: 'bg-red-500/15 text-red-400 border-red-500/30',
};

const TYPE_ICON: Record<string, React.ReactNode> = {
  Normal: <CheckCircle className="w-3.5 h-3.5" />,
  Warning: <AlertTriangle className="w-3.5 h-3.5" />,
  Error: <Info className="w-3.5 h-3.5" />,
};

function formatAge(ts: string): string {
  try {
    const diff = Date.now() - new Date(ts).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    return `${Math.floor(hours / 24)}d ago`;
  } catch { return ts; }
}

const Events: React.FC = () => {
  usePageTitle('Events');
  const [events, setEvents] = useState<K8sEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setEvents((await fetchEvents()).data.events ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch events'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const filtered = search
    ? events.filter((e) => e.message.toLowerCase().includes(search.toLowerCase()) || e.object.toLowerCase().includes(search.toLowerCase()) || e.reason.toLowerCase().includes(search.toLowerCase()))
    : events;

  const warnings = events.filter((e) => e.type === 'Warning').length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-amber-500 to-amber-700 flex items-center justify-center shadow-lg shadow-amber-500/20"><Bell className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Events</h1></div>
          <p className="text-sm text-slate-400 mt-1">Kubernetes cluster events</p>
        </div>
        <div className="flex items-center gap-2">
          <ExportButton data={filtered as Record<string, unknown>[]} filename="events" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="flex items-center gap-3 mb-4">
        <input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search events..."
          className="flex-1 px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500" />
        {warnings > 0 && <span className="px-3 py-1.5 rounded-lg bg-yellow-500/10 border border-yellow-500/30 text-yellow-400 text-sm">{warnings} warnings</span>}
      </div>

      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-amber-500 to-amber-700 flex items-center justify-center shadow-lg shadow-amber-500/20">
            <Bell className="w-4 h-4 text-white" />
          </div>
          <h2 className="text-lg font-semibold text-white">Cluster Events</h2>
        </div>
        {loading && <div className="flex justify-center p-3"><Loader2 className="w-5 h-5 animate-spin text-blue-400" /></div>}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/50 bg-slate-900/50">
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Type</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Reason</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Object</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Message</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Namespace</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Count</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Age</th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 && !loading ? (
                <tr><td colSpan={7} className="px-4 py-12 text-center text-slate-400">No events found</td></tr>
              ) : filtered.map((e) => (
                <tr key={e.id} className="border-b border-slate-700/30 table-row-hover">
                  <td className="px-4 py-2.5">
                    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border ${TYPE_BADGE[e.type] ?? ''}`}>
                      {TYPE_ICON[e.type]} {e.type}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 font-medium text-white">{e.reason}</td>
                  <td className="px-4 py-2.5 text-slate-400 font-mono text-xs">{e.object}</td>
                  <td className="px-4 py-2.5 text-white max-w-md truncate" title={e.message}>{e.message}</td>
                  <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{e.namespace}</span></td>
                  <td className="px-4 py-2.5 text-right text-slate-400">{e.count}</td>
                  <td className="px-4 py-2.5 text-right text-slate-400">{formatAge(e.last_seen ?? e.last_timestamp ?? '')}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="px-4 py-3 border-t border-slate-700/50 text-sm text-slate-400">
          Showing {filtered.length} of {events.length} events
        </div>
      </div>
    </div>
  );
};

export default Events;
