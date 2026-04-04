import React, { useState, useEffect, useCallback } from 'react';
import { CircleDot, RefreshCw, Loader2, Search } from 'lucide-react';
import { fetchEndpoints, CiliumEndpoint } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import EmptyState from '../../components/EmptyState';

const STATUS_BADGE: Record<string, string> = {
  ready: 'bg-green-500/15 text-green-400 border-green-500/30',
  not_ready: 'bg-red-500/15 text-red-400 border-red-500/30',
  disconnecting: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
};

const ENFORCEMENT_BADGE: Record<string, string> = {
  always: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  default: 'bg-slate-500/15 text-slate-400 border-slate-500/30',
  never: 'bg-red-500/15 text-red-400 border-red-500/30',
};

const Endpoints: React.FC = () => {
  usePageTitle('Endpoints');
  const [endpoints, setEndpoints] = useState<CiliumEndpoint[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setEndpoints((await fetchEndpoints()).data.endpoints ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch endpoints'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const filtered = search
    ? endpoints.filter((e) => e.name.toLowerCase().includes(search.toLowerCase()) || e.namespace.toLowerCase().includes(search.toLowerCase()) || e.labels.some((l) => l.toLowerCase().includes(search.toLowerCase())))
    : endpoints;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-green-700 flex items-center justify-center shadow-lg shadow-green-500/20"><CircleDot className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Cilium Endpoints</h1></div>
          <p className="text-sm text-slate-400 mt-1">Managed endpoints with identity and policy status</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="relative mb-4">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
        <input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search endpoints, labels..."
          className="w-full pl-9 pr-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-blue-500" />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
        {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 col-span-full mx-auto my-8" />}
        {!loading && filtered.length === 0 && (
          <div className="col-span-full">
            <EmptyState
              icon={<CircleDot className="w-8 h-8 text-slate-400" />}
              title="No endpoints found"
              description="Connect to a Cilium cluster to see managed endpoints and their identity/policy status."
            />
          </div>
        )}
        {filtered.map((ep) => (
          <div key={ep.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-center justify-between mb-3">
              <div className="font-semibold text-white truncate">{ep.name}</div>
              <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[ep.status] ?? ''}`}>{ep.status}</span>
            </div>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-slate-400">Namespace</span>
                <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{ep.namespace}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">Identity</span>
                <span className="font-mono text-white">{ep.identity}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">IPv4</span>
                <span className="font-mono text-white">{ep.ipv4}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400">Policy</span>
                <span className={`px-2 py-0.5 rounded-full text-xs border ${ENFORCEMENT_BADGE[ep.policy_enforcement] ?? ''}`}>{ep.policy_enforcement}</span>
              </div>
            </div>
            <div className="flex flex-wrap gap-1 mt-3 pt-3 border-t border-slate-700/50">
              {ep.labels.map((l) => (
                <span key={l} className="px-2 py-0.5 rounded bg-slate-900/50 text-xs text-slate-400">{l}</span>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Endpoints;
