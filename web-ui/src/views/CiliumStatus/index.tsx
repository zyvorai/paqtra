import React, { useState, useEffect, useCallback } from 'react';
import { Activity, RefreshCw, Loader2, CheckCircle, XCircle } from 'lucide-react';
import { fetchCiliumStatus, CiliumAgentStatus } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATUS_DOT: Record<string, string> = { ok: 'bg-green-400', warning: 'bg-yellow-400', failure: 'bg-red-400' };

const CiliumStatus: React.FC = () => {
  usePageTitle('Cilium Status');
  const [agents, setAgents] = useState<CiliumAgentStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setAgents((await fetchCiliumStatus()).data.agents ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500 to-cyan-700 flex items-center justify-center shadow-lg shadow-cyan-500/20"><Activity className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Cilium Agent Status</h1></div>
          <p className="text-sm text-slate-400 mt-1">Per-node Cilium agent health and configuration</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {agents.map((a) => (
          <div key={a.node} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                <span className={`w-3 h-3 rounded-full ${STATUS_DOT[a.status] ?? 'bg-gray-400'}`} />
                <span className="font-semibold text-white text-lg">{a.node}</span>
              </div>
              <span className="text-xs text-slate-400 font-mono">v{a.version}</span>
            </div>
            <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm mb-4">
              <div><span className="text-slate-400">Uptime: </span><span className="text-white">{a.uptime}</span></div>
              <div><span className="text-slate-400">Endpoints: </span><span className="text-white">{a.endpoint_count}</span></div>
              <div><span className="text-slate-400">Identities: </span><span className="text-white">{a.identity_count}</span></div>
              <div><span className="text-slate-400">Policy Rev: </span><span className="text-white font-mono">{a.policy_revision}</span></div>
              <div><span className="text-slate-400">Proxy: </span><span className="text-white">{a.proxy_redirects} redirects</span></div>
              <div className="flex items-center gap-1">
                <span className="text-slate-400">Controllers: </span>
                {(a.controllers_failing ?? 0) > 0 ? <XCircle className="w-3.5 h-3.5 text-red-400" /> : <CheckCircle className="w-3.5 h-3.5 text-green-400" />}
                <span className={(a.controllers_failing ?? 0) > 0 ? 'text-red-400' : 'text-green-400'}>{(a.controllers_total ?? 0) - (a.controllers_failing ?? 0)}/{a.controllers_total ?? 0}</span>
              </div>
            </div>
            <div className="border-t border-slate-700/50 pt-3 grid grid-cols-2 gap-2 text-xs">
              <div><span className="text-slate-400">Datapath: </span><span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-white">{a.datapath}</span></div>
              <div><span className="text-slate-400">Masquerade: </span><span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-white">{a.masquerading}</span></div>
              <div><span className="text-slate-400">Encryption: </span><span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-white">{a.encryption}</span></div>
              <div><span className="text-slate-400">KPR: </span><span className="px-1.5 py-0.5 rounded bg-slate-900/50 text-white">{a.kube_proxy_replacement}</span></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default CiliumStatus;
