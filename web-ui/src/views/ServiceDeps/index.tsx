import React, { useState, useEffect, useCallback } from 'react';
import { Workflow, RefreshCw, Loader2, ArrowRight } from 'lucide-react';
import { fetchServiceDeps, ServiceDep } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

function errorColor(rate: number): string {
  if (rate > 1) return 'text-red-400';
  if (rate > 0.5) return 'text-yellow-400';
  return 'text-green-400';
}

function latencyColor(ms: number): string {
  if (ms > 100) return 'text-red-400';
  if (ms > 50) return 'text-yellow-400';
  return 'text-green-400';
}

const ServiceDeps: React.FC = () => {
  usePageTitle('Dependencies');
  const [deps, setDeps] = useState<ServiceDep[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setDeps((await fetchServiceDeps()).data.dependencies ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const services = [...new Set(deps.flatMap((d) => [d.source, d.destination]))];

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-purple-500 to-purple-700 flex items-center justify-center shadow-lg shadow-purple-500/20"><Workflow className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Service Dependencies</h1></div>
          <p className="text-sm text-slate-400 mt-1">Service-to-service communication graph with metrics</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Service summary */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Services</div>
          <div className="text-2xl font-bold text-white">{services.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Connections</div>
          <div className="text-2xl font-bold text-white">{deps.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total RPS</div>
          <div className="text-2xl font-bold text-white">{deps.reduce((a, d) => a + d.request_rate, 0).toFixed(0)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">High Error</div>
          <div className="text-2xl font-bold text-red-400">{deps.filter((d) => d.error_rate > 1).length}</div>
        </div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Dependency cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {deps.map((d, i) => (
          <div key={i} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-center gap-3 mb-3">
              <span className="font-semibold text-white">{d.source}</span>
              <ArrowRight className="w-4 h-4 text-slate-400 flex-shrink-0" />
              <span className="font-semibold text-white">{d.destination}</span>
              <span className="ml-auto px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{d.protocol}:{d.port}</span>
            </div>
            <div className="grid grid-cols-4 gap-3 text-sm">
              <div>
                <div className="text-xs text-slate-400">RPS</div>
                <div className="font-medium text-white">{d.request_rate}</div>
              </div>
              <div>
                <div className="text-xs text-slate-400">Error Rate</div>
                <div className={`font-medium ${errorColor(d.error_rate)}`}>{d.error_rate}%</div>
              </div>
              <div>
                <div className="text-xs text-slate-400">P50 Latency</div>
                <div className={`font-medium ${latencyColor(d.latency_p50)}`}>{d.latency_p50} ms</div>
              </div>
              <div>
                <div className="text-xs text-slate-400">P99 Latency</div>
                <div className={`font-medium ${latencyColor(d.latency_p99)}`}>{d.latency_p99} ms</div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ServiceDeps;
