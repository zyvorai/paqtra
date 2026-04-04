import React, { useState, useEffect, useCallback } from 'react';
import { HeartPulse, RefreshCw, Loader2, CheckCircle, XCircle, AlertTriangle, Server, Box, CircleDot } from 'lucide-react';
import { fetchClusterHealth, ClusterHealthSummary } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATUS_ICON: Record<string, React.ReactNode> = { healthy: <CheckCircle className="w-5 h-5 text-green-400" />, degraded: <AlertTriangle className="w-5 h-5 text-yellow-400" />, unhealthy: <XCircle className="w-5 h-5 text-red-400" /> };
const STATUS_BG: Record<string, string> = { healthy: 'border-l-green-400', degraded: 'border-l-yellow-400', unhealthy: 'border-l-red-400' };

function scoreColor(s: number) { return s >= 90 ? 'text-green-400' : s >= 70 ? 'text-yellow-400' : 'text-red-400'; }

const ClusterHealth: React.FC = () => {
  usePageTitle('Cluster Health');
  const [health, setHealth] = useState<ClusterHealthSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setHealth((await fetchClusterHealth()).data); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); const i = setInterval(fetchData, 30000); return () => clearInterval(i); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-rose-500 to-rose-700 flex items-center justify-center shadow-lg shadow-rose-500/20"><HeartPulse className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Cluster Health</h1></div>
          <p className="text-sm text-slate-400 mt-1">Comprehensive cluster health dashboard</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && !health && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {health && (
        <>
          {/* Score + overview */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 flex flex-col items-center">
              <div className="relative w-24 h-24 mb-3">
                <svg className="w-full h-full -rotate-90" viewBox="0 0 100 100">
                  <circle cx="50" cy="50" r="42" fill="none" stroke="#334155" strokeWidth="6" />
                  <circle cx="50" cy="50" r="42" fill="none" stroke={(health.score ?? health.health_score ?? 0) >= 90 ? '#22c55e' : (health.score ?? health.health_score ?? 0) >= 70 ? '#eab308' : '#ef4444'} strokeWidth="6" strokeDasharray={`${((health.score ?? health.health_score ?? 0) / 100) * 263.9} 263.9`} strokeLinecap="round" />
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className={`text-2xl font-bold ${scoreColor(health.score ?? health.health_score ?? 0)}`}>{health.score ?? health.health_score ?? 0}</span>
                </div>
              </div>
              <span className={`px-3 py-1 rounded-full text-sm border ${(health.overall ?? health.status ?? 'unknown') === 'healthy' ? 'bg-green-500/15 text-green-400 border-green-500/30' : 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30'}`}>{health.overall ?? health.status ?? 'unknown'}</span>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue flex flex-col justify-center">
              <div className="flex items-center gap-2 text-slate-400 mb-1"><Server className="w-4 h-4" /><span className="text-xs">Nodes</span></div>
              <div className="text-2xl font-bold text-white">{health.node_count ?? health.kubernetes?.nodes ?? 0}</div>
              <div className="text-xs text-slate-400 mt-1">K8s {health.kubernetes_version ?? health.kubernetes?.version ?? '-'}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green flex flex-col justify-center">
              <div className="flex items-center gap-2 text-slate-400 mb-1"><Box className="w-4 h-4" /><span className="text-xs">Pods</span></div>
              <div className="text-2xl font-bold text-white">{health.pod_count ?? health.kubernetes?.pods ?? 0}</div>
              <div className="text-xs text-slate-400 mt-1">Cilium {health.cilium_version ?? health.cilium?.version ?? '-'}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple flex flex-col justify-center">
              <div className="flex items-center gap-2 text-slate-400 mb-1"><CircleDot className="w-4 h-4" /><span className="text-xs">Endpoints</span></div>
              <div className="text-2xl font-bold text-white">{health.endpoint_count ?? health.kubernetes?.endpoints ?? 0}</div>
            </div>
          </div>

          {/* Components */}
          <div className="flex items-center gap-2 mb-3"><div className="w-1 h-5 bg-gradient-to-b from-green-400 to-emerald-500 rounded-full" /><h2 className="text-lg font-semibold text-white">Component Status</h2></div>
          <div className="space-y-2">
            {(health.components ?? []).map((c) => (
              <div key={c.name} className={`rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 border-l-4 ${STATUS_BG[c.status] ?? 'border-l-border'}`}>
                <div className="flex items-center gap-3">
                  {STATUS_ICON[c.status] ?? STATUS_ICON.healthy}
                  <div className="flex-1">
                    <div className="font-medium text-white">{c.name}</div>
                    <div className="text-sm text-slate-400">{c.message}</div>
                  </div>
                  <div className="text-right text-xs text-slate-400">
                    <div>Uptime: {c.uptime ?? '-'}</div>
                    <div>Checked: {c.last_check ? new Date(c.last_check).toLocaleTimeString() : '-'}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
};

export default ClusterHealth;
