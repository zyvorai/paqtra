import React, { useState, useEffect, useCallback } from 'react';
import { Scale, RefreshCw, Loader2, ChevronDown, ChevronRight } from 'lucide-react';
import { fetchLBServices, LBService } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATE_BADGE: Record<string, string> = { active: 'bg-green-500/15 text-green-400 border-green-500/30', draining: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30', inactive: 'bg-red-500/15 text-red-400 border-red-500/30' };

const LoadBalancer: React.FC = () => {
  usePageTitle('Load Balancer');
  const [services, setServices] = useState<LBService[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setServices((await fetchLBServices()).data.services ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20"><Scale className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Load Balancer</h1></div>
          <p className="text-sm text-slate-400 mt-1">Cilium L4/L7 load balancer service status</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="space-y-3">
        {services.map((s) => (
          <div key={s.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
            <button onClick={() => setExpanded(expanded === s.name ? null : s.name)} className="w-full flex items-center gap-4 px-5 py-4 text-left hover:bg-slate-700/30/30 transition-colors">
              {expanded === s.name ? <ChevronDown className="w-4 h-4 text-slate-400" /> : <ChevronRight className="w-4 h-4 text-slate-400" />}
              <div className="flex-1">
                <div className="font-semibold text-white">{s.namespace}/{s.name}</div>
                <div className="text-xs text-slate-400 mt-0.5">{s.frontend_ip ?? s.frontend ?? '-'}:{s.frontend_port ?? 0} &bull; {s.protocol ?? '-'} &bull; {s.algorithm ?? 'round-robin'}</div>
              </div>
              <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{s.type ?? 'ClusterIP'}</span>
              <span className="text-xs text-slate-400">{(s.backends ?? []).length} backends</span>
              {(s.session_affinity && s.session_affinity !== 'none' && s.session_affinity !== 'false') && <span className="px-2 py-0.5 rounded bg-blue-500/15 text-blue-400 text-xs border border-blue-500/30">sticky</span>}
            </button>
            {expanded === s.name && (
              <div className="border-t border-slate-700/50 px-5 py-3 animate-fade-in">
                <table className="w-full text-sm">
                  <thead><tr className="text-slate-400 text-xs">
                    <th className="text-left pb-2">Backend IP</th><th className="text-left pb-2">Port</th><th className="text-left pb-2">Weight</th><th className="text-left pb-2">State</th>
                  </tr></thead>
                  <tbody>{(s.backends ?? []).map((b, i) => (
                    <tr key={i} className="border-t border-slate-700/30">
                      <td className="py-1.5 font-mono text-white">{b.ip ?? b.address ?? '-'}</td>
                      <td className="py-1.5 text-white">{b.port}</td>
                      <td className="py-1.5 text-white">{b.weight}</td>
                      <td className="py-1.5"><span className={`px-2 py-0.5 rounded-full text-xs border ${STATE_BADGE[b.state] ?? ''}`}>{b.state}</span></td>
                    </tr>
                  ))}</tbody>
                </table>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default LoadBalancer;
