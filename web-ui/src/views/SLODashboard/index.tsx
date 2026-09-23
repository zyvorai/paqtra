import React, { useState, useCallback } from 'react';
import { Target, Loader2, CheckCircle, AlertTriangle, XCircle } from 'lucide-react';
import { fetchSLOs, SLOTarget } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

function statusIcon(s: string) {
  if (s === 'met') return <CheckCircle className="w-5 h-5 text-green-400" />;
  if (s === 'at_risk') return <AlertTriangle className="w-5 h-5 text-yellow-400" />;
  return <XCircle className="w-5 h-5 text-red-400" />;
}

function budgetColor(remaining: number, total: number) {
  const pct = (remaining / total) * 100;
  return pct > 50 ? 'bg-green-400' : pct > 20 ? 'bg-yellow-400' : 'bg-red-400';
}

const SLODashboard: React.FC = () => {
  usePageTitle('SLO Dashboard');
  const [slos, setSlos] = useState<SLOTarget[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setSlos((await fetchSLOs()).data.slos ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const met = slos.filter((s) => s.status === 'met').length;
  const breached = slos.filter((s) => s.status === 'breached').length;

  return (
    <div className="netra-page">
      <div className="page-chrome flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-green-700 flex items-center justify-center shadow-lg shadow-green-500/20"><Target className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">SLO Dashboard</h1></div>
          <p className="text-sm text-slate-400 mt-1">Service Level Objective tracking and error budget</p>
        </div>
        <div className="flex items-center gap-3">
          <ExportButton data={slos as unknown as Record<string, unknown>[]} filename="slo-dashboard" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Total SLOs</div><div className="text-2xl font-bold text-white">{slos.length}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Meeting Target</div><div className="text-2xl font-bold text-green-400">{met}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Breached</div><div className="text-2xl font-bold text-red-400">{breached}</div></div>
      </div>

      {loading && slos.length === 0 && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {!loading && slos.length === 0 && !error && (
        <div className="text-center py-12 text-slate-400">No SLO targets configured.</div>
      )}

      <div className="space-y-4">
        {slos.map((s) => {
          const budgetPct = s.budget_total > 0 ? (s.budget_remaining / s.budget_total) * 100 : 0;
          return (
            <div key={s.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
              <div className="flex items-center gap-3 mb-4">
                {statusIcon(s.status)}
                <div className="flex-1">
                  <div className="font-semibold text-white">{s.name}</div>
                  <div className="text-xs text-slate-400">{s.service} &bull; {s.metric} &bull; {s.window}</div>
                </div>
                <div className="text-right">
                  <div className="text-2xl font-bold text-white">{(s.current ?? 0).toFixed(2)}%</div>
                  <div className="text-xs text-slate-400">Target: {s.target}%</div>
                </div>
              </div>
              <div className="mb-2">
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-slate-400">Error Budget</span>
                  <span className="text-white">{(s.budget_remaining ?? 0).toFixed(1)} / {(s.budget_total ?? 0).toFixed(1)} min remaining</span>
                </div>
                <div className="w-full h-2.5 rounded-full bg-slate-700 overflow-hidden">
                  <div className={`h-full rounded-full ${budgetColor(s.budget_remaining, s.budget_total)} transition-all`} style={{ width: `${Math.max(budgetPct, 0)}%` }} />
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default SLODashboard;
