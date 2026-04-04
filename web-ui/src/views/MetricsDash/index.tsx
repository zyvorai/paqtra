import React, { useState, useEffect, useCallback } from 'react';
import { BarChart3, RefreshCw, Loader2, Activity, AlertTriangle, Database, Clock } from 'lucide-react';
import { fetchMetricsSummary, MetricsSummary } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

const MetricsDash: React.FC = () => {
  usePageTitle('API Metrics');
  const [metrics, setMetrics] = useState<MetricsSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setMetrics((await fetchMetricsSummary()).data); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); const i = setInterval(fetchData, 10000); return () => clearInterval(i); }, [fetchData]);

  const cacheHitRate = metrics && (metrics.cache_hits + metrics.cache_misses) > 0
    ? ((metrics.cache_hits / (metrics.cache_hits + metrics.cache_misses)) * 100).toFixed(1) : '0.0';
  const errorRate = metrics && (metrics.total_requests ?? metrics.request_count ?? 0) > 0
    ? (((metrics.total_errors ?? metrics.error_count ?? 0) / (metrics.total_requests ?? metrics.request_count ?? 0)) * 100).toFixed(2) : '0.00';

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-purple-500 to-purple-700 flex items-center justify-center shadow-lg shadow-purple-500/20"><BarChart3 className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">API Metrics</h1></div>
          <p className="text-sm text-slate-400 mt-1">Cilium Vision API server performance metrics</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {loading && !metrics && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {metrics && (
        <>
          {/* Main stats */}
          <div className="grid grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
            <div className="rounded-xl border border-slate-700/50 p-5 stat-card-blue card-glow transition-all hover:scale-[1.02]">
              <div className="flex items-center gap-2 text-slate-400 mb-2"><Activity className="w-5 h-5" /> <span className="text-xs">Total Requests</span></div>
              <div className="text-3xl font-bold text-white">{((metrics.total_requests ?? metrics.request_count ?? 0)).toLocaleString()}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-5 stat-card-red card-glow transition-all hover:scale-[1.02]">
              <div className="flex items-center gap-2 text-slate-400 mb-2"><AlertTriangle className="w-5 h-5" /> <span className="text-xs">Total Errors</span></div>
              <div className="text-3xl font-bold text-red-400">{((metrics.total_errors ?? metrics.error_count ?? 0)).toLocaleString()}</div>
              <div className="text-sm text-slate-400 mt-1">{errorRate}% error rate</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-5 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
              <div className="flex items-center gap-2 text-slate-400 mb-2"><Clock className="w-5 h-5" /> <span className="text-xs">Uptime</span></div>
              <div className="text-3xl font-bold text-green-400">{formatUptime(metrics.uptime_seconds)}</div>
            </div>
          </div>

          {/* Detailed metrics */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Cache */}
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
              <h3 className="text-sm font-semibold text-white mb-4 flex items-center gap-2"><Database className="w-4 h-4" /> Cache Performance</h3>
              <div className="grid grid-cols-3 gap-4 mb-4">
                <div><div className="text-xs text-slate-400">Hits</div><div className="text-xl font-bold text-green-400">{(metrics.cache_hits ?? 0).toLocaleString()}</div></div>
                <div><div className="text-xs text-slate-400">Misses</div><div className="text-xl font-bold text-yellow-400">{(metrics.cache_misses ?? 0).toLocaleString()}</div></div>
                <div><div className="text-xs text-slate-400">Hit Rate</div><div className="text-xl font-bold text-white">{cacheHitRate}%</div></div>
              </div>
              <div className="w-full h-3 rounded-full bg-slate-700 overflow-hidden">
                <div className="h-full rounded-full bg-green-400 transition-all" style={{ width: `${cacheHitRate}%` }} />
              </div>
            </div>

            {/* Queries */}
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
              <h3 className="text-sm font-semibold text-white mb-4 flex items-center gap-2"><Activity className="w-4 h-4" /> Backend Queries</h3>
              <div className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-slate-400">Total Queries</span>
                    <span className="font-medium text-white">{((metrics.total_queries ?? 0)).toLocaleString()}</span>
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-4 pt-3 border-t border-slate-700/50">
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">{(metrics.total_requests ?? metrics.request_count ?? 0) > 0 ? ((metrics.total_queries ?? 0) / (metrics.total_requests ?? metrics.request_count ?? 0)).toFixed(2) : '0'}</div>
                    <div className="text-xs text-slate-400 mt-1">Queries per Request</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">{(metrics.total_requests ?? metrics.request_count ?? 0) > 0 ? ((metrics.total_requests ?? metrics.request_count ?? 0) / Math.max(metrics.uptime_seconds, 1)).toFixed(1) : '0'}</div>
                    <div className="text-xs text-slate-400 mt-1">Requests per Second</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default MetricsDash;
