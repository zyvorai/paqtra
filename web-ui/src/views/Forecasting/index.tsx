import React, { useState, useEffect, useCallback } from 'react';
import { TrendingUp, RefreshCw, Loader2, Lightbulb } from 'lucide-react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { fetchForecast, fetchForecastMetrics, ForecastResult } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const METRIC_LABELS: Record<string, string> = { cpu_usage: 'CPU Usage', memory_usage: 'Memory Usage', network_throughput: 'Network Throughput', pod_count: 'Pod Count' };

const Forecasting: React.FC = () => {
  usePageTitle('Forecasting');
  const [metrics, setMetrics] = useState<string[]>([]);
  const [selected, setSelected] = useState('cpu_usage');
  const [result, setResult] = useState<ForecastResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchMetrics = useCallback(async () => {
    try { setMetrics((await fetchForecastMetrics()).data.metrics ?? []); }
    catch { /* ignore */ }
  }, []);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setResult((await fetchForecast(selected)).data); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, [selected]);

  useEffect(() => { fetchMetrics(); }, [fetchMetrics]);
  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-green-700 flex items-center justify-center shadow-lg shadow-green-500/20"><TrendingUp className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Capacity Forecasting</h1></div>
          <p className="text-sm text-slate-400 mt-1">Predictive resource forecasting with confidence intervals</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Metric selector */}
      <div className="flex gap-1 mb-6 p-1 rounded-lg bg-slate-900/50 w-fit">
        {metrics.map((m) => (
          <button key={m} onClick={() => setSelected(m)} className={`px-4 py-2 rounded-md text-sm transition-colors ${selected === m ? 'bg-slate-800/50 text-white shadow' : 'text-slate-400 hover:text-white'}`}>
            {METRIC_LABELS[m] ?? m}
          </button>
        ))}
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {result && (
        <>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 mb-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-green-500 to-green-700 flex items-center justify-center shadow-lg shadow-green-500/20">
                  <TrendingUp className="w-4 h-4 text-white" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-white">{METRIC_LABELS[result.metric] ?? result.metric}</h2>
                  <p className="text-xs text-slate-400">Unit: {result.unit}</p>
                </div>
              </div>
              <div className="flex items-center gap-4 text-xs">
                <span className="flex items-center gap-1"><span className="w-3 h-0.5 bg-blue-400 inline-block" /> Actual</span>
                <span className="flex items-center gap-1"><span className="w-3 h-0.5 bg-green-400 inline-block" /> Predicted</span>
                <span className="flex items-center gap-1"><span className="w-3 h-0.5 bg-green-400/30 inline-block" /> Confidence</span>
              </div>
            </div>
            <ResponsiveContainer width="100%" height={350}>
              <AreaChart data={result.points}>
                <defs>
                  <linearGradient id="confGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#22c55e" stopOpacity={0.2} />
                    <stop offset="95%" stopColor="#22c55e" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="timestamp" tick={{ fill: '#94a3b8', fontSize: 10 }} />
                <YAxis tick={{ fill: '#94a3b8', fontSize: 11 }} />
                <Tooltip contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #334155', borderRadius: 8 }} />
                <Area type="monotone" dataKey="upper_bound" stroke="none" fill="url(#confGrad)" name="Upper Bound" />
                <Area type="monotone" dataKey="lower_bound" stroke="none" fill="transparent" name="Lower Bound" />
                <Area type="monotone" dataKey="predicted" stroke="#22c55e" fill="none" strokeDasharray="5 3" name="Predicted" />
                <Area type="monotone" dataKey="actual" stroke="#3b82f6" fill="none" strokeWidth={2} name="Actual" />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {result.recommendation && (
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 flex items-start gap-3">
              <Lightbulb className="w-5 h-5 text-yellow-400 mt-0.5 flex-shrink-0" />
              <div>
                <div className="text-sm font-medium text-white mb-1">Recommendation</div>
                <div className="text-sm text-slate-400">{result.recommendation}</div>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default Forecasting;
