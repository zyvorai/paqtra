import React, { useState, useEffect, useCallback } from 'react';
import { BellRing, RefreshCw, Loader2, Bell, BellOff, Clock } from 'lucide-react';
import { fetchAlertRules, fetchAlertHistory, AlertRule, AlertEvent } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const SEV_BADGE: Record<string, string> = { critical: 'bg-red-500/15 text-red-400 border-red-500/30', high: 'bg-orange-500/15 text-orange-400 border-orange-500/30', warning: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30' };
const STATUS_BADGE: Record<string, string> = { firing: 'bg-red-500/15 text-red-400 border-red-500/30', resolved: 'bg-green-500/15 text-green-400 border-green-500/30' };

const Alerts: React.FC = () => {
  usePageTitle('Alerts');
  const [rules, setRules] = useState<AlertRule[]>([]);
  const [history, setHistory] = useState<AlertEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<'rules' | 'history'>('rules');

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { const [r, h] = await Promise.all([fetchAlertRules(), fetchAlertHistory()]); setRules(r.data.rules ?? []); setHistory(h.data.alerts ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const firing = history.filter((a) => a.status === 'firing').length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-amber-500 to-amber-700 flex items-center justify-center shadow-lg shadow-amber-500/20"><BellRing className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Alerts</h1></div>
          <p className="text-sm text-slate-400 mt-1">Alert rules and notification management</p>
        </div>
        <div className="flex items-center gap-3">
          {firing > 0 && <span className="px-3 py-1.5 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm animate-pulse-dot">{firing} firing</span>}
          <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      <div className="flex gap-1 mb-4 p-1 rounded-lg bg-slate-900/50 w-fit">
        <button onClick={() => setTab('rules')} className={`px-4 py-2 rounded-md text-sm transition-colors ${tab === 'rules' ? 'bg-slate-800/50 text-white shadow' : 'text-slate-400'}`}>Rules ({rules.length})</button>
        <button onClick={() => setTab('history')} className={`px-4 py-2 rounded-md text-sm transition-colors ${tab === 'history' ? 'bg-slate-800/50 text-white shadow' : 'text-slate-400'}`}>History ({history.length})</button>
      </div>
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}
      {tab === 'rules' && (
        <div className="space-y-3">
          {rules.map((r) => (
            <div key={r.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 flex items-center gap-4">
              {r.enabled ? <Bell className="w-5 h-5 text-blue-400" /> : <BellOff className="w-5 h-5 text-slate-400" />}
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-white">{r.name}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${SEV_BADGE[r.severity] ?? ''}`}>{r.severity}</span>
                  {!r.enabled && <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">disabled</span>}
                </div>
                <div className="text-sm text-slate-400 font-mono">{r.condition}</div>
                <div className="flex items-center gap-3 mt-1 text-xs text-slate-400">
                  <span>Triggered {r.trigger_count}x</span>
                  {r.last_triggered && <span className="flex items-center gap-1"><Clock className="w-3 h-3" /> {new Date(r.last_triggered).toLocaleDateString()}</span>}
                  <span>Channels: {r.channels.join(', ')}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
      {tab === 'history' && (
        <div className="space-y-3">
          {history.map((a) => (
            <div key={a.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
              <div className="flex items-center gap-2 mb-2">
                <span className={`px-2 py-0.5 rounded-full text-xs border ${SEV_BADGE[a.severity] ?? ''}`}>{a.severity}</span>
                <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[a.status] ?? ''}`}>{a.status}</span>
                <span className="font-medium text-white">{a.rule_name}</span>
                <span className="ml-auto text-xs text-slate-400">{new Date(a.fired_at ?? a.timestamp).toLocaleString()}</span>
              </div>
              <div className="text-sm text-slate-400">{a.message}</div>
              {a.resolved_at && <div className="text-xs text-green-400 mt-1">Resolved: {new Date(a.resolved_at).toLocaleString()}</div>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default Alerts;
