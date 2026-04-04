import React, { useState, useEffect, useCallback } from 'react';
import { Fingerprint, RefreshCw, Loader2, Search, Shield, Tag } from 'lucide-react';
import { fetchIdentities, CiliumIdentity } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const Identities: React.FC = () => {
  usePageTitle('Identities');
  const [identities, setIdentities] = useState<CiliumIdentity[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setIdentities((await fetchIdentities()).data.identities ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const filtered = search ? identities.filter((i) => i.labels.some((l) => l.toLowerCase().includes(search.toLowerCase())) || i.namespace.includes(search) || String(i.id).includes(search)) : identities;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-violet-500 to-violet-700 flex items-center justify-center shadow-lg shadow-violet-500/20"><Fingerprint className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Security Identities</h1></div>
          <p className="text-sm text-slate-400 mt-1">Cilium security identity management</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      <div className="grid grid-cols-3 gap-3 mb-4">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Identities</div><div className="text-2xl font-bold text-white">{identities.length}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Endpoints</div><div className="text-2xl font-bold text-white">{identities.reduce((a, i) => a + i.endpoints_count, 0)}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">With Policies</div><div className="text-2xl font-bold text-white">{identities.filter((i) => i.policy_count > 0).length}</div></div>
      </div>
      <div className="relative mb-4"><Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" /><input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search identities, labels..." className="w-full pl-9 pr-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500" /></div>
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {filtered.map((i) => (
          <div key={i.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
            <div className="flex items-center justify-between mb-3">
              <span className="text-lg font-bold text-blue-400 font-mono">#{i.id}</span>
              <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{i.namespace}</span>
            </div>
            <div className="flex flex-wrap gap-1 mb-3">
              {i.labels.map((l) => (
                <span key={l} className="flex items-center gap-1 px-2 py-0.5 rounded bg-slate-900/50 text-xs text-slate-400"><Tag className="w-3 h-3" />{l}</span>
              ))}
            </div>
            <div className="flex gap-4 text-sm border-t border-slate-700/50 pt-3">
              <div className="flex items-center gap-1 text-slate-400"><Fingerprint className="w-3.5 h-3.5" /> {i.endpoints_count} endpoints</div>
              <div className="flex items-center gap-1 text-slate-400"><Shield className="w-3.5 h-3.5" /> {i.policy_count} policies</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Identities;
