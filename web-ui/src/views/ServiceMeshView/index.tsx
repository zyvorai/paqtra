import React, { useState, useEffect, useCallback } from 'react';
import { Hexagon, RefreshCw, Loader2, Lock, LockOpen, RotateCcw, Timer, Zap } from 'lucide-react';
import { fetchMeshServices, MeshService } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const ServiceMeshView: React.FC = () => {
  usePageTitle('Service Mesh');
  const [services, setServices] = useState<MeshService[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setServices((await fetchMeshServices()).data.services ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const mtlsCount = services.filter((s) => s.mtls).length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20"><Hexagon className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Service Mesh</h1></div>
          <p className="text-sm text-slate-400 mt-1">Cilium service mesh configuration and traffic policies</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Services</div><div className="text-2xl font-bold text-white">{services.length}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">mTLS Enabled</div><div className="text-2xl font-bold text-green-400">{mtlsCount}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Circuit Breakers</div><div className="text-2xl font-bold text-white">{services.filter((s) => s.circuit_breaker).length}</div></div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {services.map((s) => (
          <div key={`${s.namespace}/${s.name}`} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
            <div className="flex items-center justify-between mb-3">
              <span className="font-semibold text-white">{s.name}</span>
              <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{s.namespace}</span>
            </div>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="flex items-center gap-1">{s.mtls ? <Lock className="w-3.5 h-3.5 text-green-400" /> : <LockOpen className="w-3.5 h-3.5 text-yellow-400" />}<span className={s.mtls ? 'text-green-400' : 'text-yellow-400'}>{s.mtls ? 'mTLS' : 'Plain'}</span></div>
              <div className="flex items-center gap-1"><span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-xs">{s.protocol}</span></div>
              <div className="flex items-center gap-1"><RotateCcw className="w-3.5 h-3.5 text-slate-400" /><span className="text-white">{s.retries} retries</span></div>
              <div className="flex items-center gap-1"><Timer className="w-3.5 h-3.5 text-slate-400" /><span className="text-white">{s.timeout_ms}ms</span></div>
              <div className="flex items-center gap-1"><Zap className="w-3.5 h-3.5 text-slate-400" /><span className={s.circuit_breaker ? 'text-green-400' : 'text-slate-400'}>{s.circuit_breaker ? 'CB On' : 'CB Off'}</span></div>
              <div className="text-xs text-slate-400">{s.traffic_policy}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default ServiceMeshView;
