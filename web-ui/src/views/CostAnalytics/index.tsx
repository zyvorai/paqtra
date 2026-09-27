import React, { useState, useCallback } from 'react';
import { DollarSign, Loader2, TrendingUp, TrendingDown } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { fetchCostBreakdown, CostBreakdown, CostSummary } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';
import PagePulse from '../../components/kit/PagePulse';
import { useChanged } from '../../components/kit/useSeries';

const CostAnalytics: React.FC = () => {
  usePageTitle('Cost Analytics');
  const [costs, setCosts] = useState<CostBreakdown[]>([]);
  const [summary, setSummary] = useState<CostSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { const res = await fetchCostBreakdown(); const d = res.data as Record<string, unknown>; setCosts((d.costs ?? d.namespaces ?? []) as typeof costs); setSummary(d.summary as typeof summary); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const tick = useChanged(summary);
  return (
    <div className="netra-page">
      <div className="page-chrome flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-green-500/20"><DollarSign className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Cost Analytics</h1></div>
          <p className="text-sm text-slate-400 mt-1">Network infrastructure cost breakdown by namespace</p>
        </div>
        <div className="flex items-center gap-3">
          <ExportButton data={costs as unknown as Record<string, unknown>[]} filename="cost-analytics" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      <PagePulse
        headline={tick ? (summary ? String(summary.note ?? 'Resource-proportional cost breakdown.') : undefined) : undefined}
        tick={tick}
        error={error || undefined}
        figures={[
          { label: 'namespaces', value: tick ? Number(summary?.total_namespaces ?? costs.length) : undefined },
          { label: 'pods', value: tick ? (summary?.total_pods != null ? Number(summary.total_pods) : '—') : undefined },
          { label: 'CPU requested', value: tick ? (summary?.total_cpu_request_millicores != null ? `${(Number(summary.total_cpu_request_millicores) / 1000).toFixed(1)} cores` : '—') : undefined },
          { label: 'memory requested', value: tick ? (summary?.total_memory_request_mib != null ? `${(Number(summary.total_memory_request_mib) / 1024).toFixed(1)} GiB` : '—') : undefined },
        ]}
      />


      {loading && costs.length === 0 && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {!loading && costs.length === 0 && !error && (
        <div className="text-center py-12 text-slate-400">No cost data available.</div>
      )}

      {costs.length > 0 && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 mb-6">
          <div className="flex items-center gap-3 mb-4">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-500 to-green-700 flex items-center justify-center shadow-lg shadow-emerald-500/20">
              <DollarSign className="w-4 h-4 text-white" />
            </div>
            <h2 className="text-base font-semibold text-white">Cost by Namespace</h2>
          </div>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={costs}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis dataKey="namespace" tick={{ fill: '#94a3b8', fontSize: 11 }} />
              <YAxis tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={(v) => `$${v}`} />
              <Tooltip contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #334155', borderRadius: 8 }} formatter={(v) => `$${Number(v ?? 0).toFixed(2)}`} />
              <Legend />
              <Bar dataKey="cpu_cost" name="CPU" fill="#3b82f6" radius={[2, 2, 0, 0]} />
              <Bar dataKey="memory_cost" name="Memory" fill="#a855f7" radius={[2, 2, 0, 0]} />
              <Bar dataKey="network_cost" name="Network" fill="#22c55e" radius={[2, 2, 0, 0]} />
              <Bar dataKey="storage_cost" name="Storage" fill="#f97316" radius={[2, 2, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Cost table */}
      {costs.length > 0 && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
          <table className="w-full text-sm">
            <thead><tr className="border-b border-slate-700/50 bg-slate-900/50">
              <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Namespace</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">CPU</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Memory</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Network</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Storage</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Total</th>
              <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Trend</th>
            </tr></thead>
            <tbody>{costs.map((c) => (
              <tr key={c.namespace} className="border-b border-slate-700/30 table-row-hover">
                <td className="px-4 py-2.5 font-medium text-white">{c.namespace}</td>
                <td className="px-4 py-2.5 text-right text-slate-400">${(c.cpu_cost ?? 0).toFixed(2)}</td>
                <td className="px-4 py-2.5 text-right text-slate-400">${(c.memory_cost ?? 0).toFixed(2)}</td>
                <td className="px-4 py-2.5 text-right text-slate-400">${(c.network_cost ?? 0).toFixed(2)}</td>
                <td className="px-4 py-2.5 text-right text-slate-400">${(c.storage_cost ?? 0).toFixed(2)}</td>
                <td className="px-4 py-2.5 text-right font-medium text-white">${(c.total_cost ?? 0).toFixed(2)}</td>
                <td className="px-4 py-2.5 text-right">
                  {(() => { const trendVal = typeof c.trend === 'number' ? c.trend : parseFloat(String(c.trend ?? "0")) || 0; return (
                  <span className={`flex items-center justify-end gap-1 ${trendVal > 0 ? 'text-red-400' : 'text-green-400'}`}>
                    {trendVal > 0 ? <TrendingUp className="w-3 h-3" /> : <TrendingDown className="w-3 h-3" />}{Math.abs(trendVal)}%
                  </span>
                  ); })()}
                </td>
              </tr>
            ))}</tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default CostAnalytics;
