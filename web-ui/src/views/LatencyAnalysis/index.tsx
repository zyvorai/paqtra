import React, { useState, useEffect, useCallback } from 'react';
import { Timer, RefreshCw, Loader2 } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { fetchLatencyAnalysis, LatencyBreakdown } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

function latColor(ms: number): string { return ms > 100 ? 'text-red-400' : ms > 50 ? 'text-yellow-400' : 'text-green-400'; }

const LatencyAnalysis: React.FC = () => {
  usePageTitle('Latency Analysis');
  const [services, setServices] = useState<LatencyBreakdown[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<LatencyBreakdown | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { const s = (await fetchLatencyAnalysis()).data.services ?? []; setServices(s); if (s.length > 0 && !selected) setSelected(s[0]); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, [selected]);

  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-orange-500 to-orange-700 flex items-center justify-center shadow-lg shadow-orange-500/20"><Timer className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Latency Analysis</h1></div>
          <p className="text-sm text-slate-400 mt-1">Per-service latency percentile breakdown</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Percentile table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden mb-6">
        <table className="w-full text-sm">
          <thead><tr className="border-b border-slate-700/50 bg-slate-900/50">
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Service</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">P50</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">P90</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">P95</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">P99</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Max</th>
          </tr></thead>
          <tbody>{services.map((s) => (
            <tr key={s.service} className={`border-b border-slate-700/30 cursor-pointer transition-colors ${selected?.service === s.service ? 'bg-blue-500/10' : 'table-row-hover'}`} onClick={() => setSelected(s)}>
              <td className="px-4 py-2.5 font-medium text-white">{s.service}</td>
              <td className={`px-4 py-2.5 text-right font-mono ${latColor(s.p50 ?? s.p50_ms ?? 0)}`}>{s.p50 ?? s.p50_ms ?? 0} ms</td>
              <td className={`px-4 py-2.5 text-right font-mono ${latColor(s.p90 ?? s.p90_ms ?? 0)}`}>{s.p90 ?? s.p90_ms ?? 0} ms</td>
              <td className={`px-4 py-2.5 text-right font-mono ${latColor(s.p95 ?? s.p95_ms ?? 0)}`}>{s.p95 ?? s.p95_ms ?? 0} ms</td>
              <td className={`px-4 py-2.5 text-right font-mono ${latColor(s.p99 ?? s.p99_ms ?? 0)}`}>{s.p99 ?? s.p99_ms ?? 0} ms</td>
              <td className={`px-4 py-2.5 text-right font-mono ${latColor(s.max ?? s.max_ms ?? 0)}`}>{s.max ?? s.max_ms ?? 0} ms</td>
            </tr>
          ))}</tbody>
        </table>
      </div>

      {/* Histogram */}
      {selected && (selected.histogram ?? []).length > 0 && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
          <h2 className="text-sm font-semibold text-white mb-4">Latency Distribution: {selected.service}</h2>
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={(selected.histogram ?? []).map(h => ({ ...h, range: h.range ?? h.bucket ?? '' }))}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis dataKey="range" tick={{ fill: '#94a3b8', fontSize: 11 }} />
              <YAxis tick={{ fill: '#94a3b8', fontSize: 11 }} />
              <Tooltip contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #334155', borderRadius: 8 }} />
              <Bar dataKey="count" fill="#3b82f6" radius={[4, 4, 0, 0]} name="Requests" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
};

export default LatencyAnalysis;
