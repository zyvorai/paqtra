import React, { useState, useEffect, useCallback } from 'react';
import { DollarSign, RefreshCw, Loader2, TrendingUp, TrendingDown, Lightbulb } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';
import { fetchCostBreakdown, CostBreakdown, CostSummary } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const CostAnalytics: React.FC = () => {
  usePageTitle('Cost Analytics');
  const [costs, setCosts] = useState<CostBreakdown[]>([]);
  const [summary, setSummary] = useState<CostSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { const res = await fetchCostBreakdown(); const d = res.data as Record<string, unknown>; setCosts((d.costs ?? d.namespaces ?? []) as typeof costs); setSummary(d.summary as typeof summary); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-green-500/20"><DollarSign className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Cost Analytics</h1></div>
          <p className="text-sm text-slate-400 mt-1">Network infrastructure cost breakdown by namespace</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {summary && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Monthly Cost</div>
            <div className="text-2xl font-bold text-white">${(summary.total_monthly ?? summary.total_monthly_cost ?? 0).toLocaleString()}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
            <div className="text-xs text-slate-400 mb-1">Daily Cost</div>
            <div className="text-2xl font-bold text-white">${(summary.total_daily ?? (summary.total_monthly_cost ? summary.total_monthly_cost / 30 : 0)).toFixed(0)}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]">
            <div className="flex items-center gap-1 text-xs text-slate-400 mb-1">Trend {(summary.cost_trend ?? summary.trend ?? '-') === 'decreasing' ? <TrendingDown className="w-3 h-3 text-green-400" /> : <TrendingUp className="w-3 h-3 text-red-400" />}</div>
            <div className={`text-2xl font-bold ${(summary.cost_trend ?? summary.trend ?? '-') === 'decreasing' ? 'text-green-400' : 'text-red-400'}`}>{summary.cost_trend ?? summary.trend ?? '-'}</div>
          </div>
          <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
            <div className="flex items-center gap-1 text-xs text-slate-400 mb-1"><Lightbulb className="w-3 h-3" /> Savings Potential</div>
            <div className="text-2xl font-bold text-green-400">${(summary.savings_potential ?? 0).toLocaleString()}</div>
          </div>
        </div>
      )}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

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
              <Tooltip contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #334155', borderRadius: 8 }} formatter={(v: number) => `$${v.toFixed(2)}`} />
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
    </div>
  );
};

export default CostAnalytics;
