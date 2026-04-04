import React, { useState, useEffect, useCallback } from 'react';
import { Route, RefreshCw, Loader2, Wifi, WifiOff } from 'lucide-react';
import { fetchBgpPeers, BgpPeer } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATE_BADGE: Record<string, string> = { Established: 'bg-green-500/15 text-green-400 border-green-500/30', Idle: 'bg-red-500/15 text-red-400 border-red-500/30', Active: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30', OpenSent: 'bg-blue-500/15 text-blue-400 border-blue-500/30' };

const BgpPeering: React.FC = () => {
  usePageTitle('BGP Peering');
  const [peers, setPeers] = useState<BgpPeer[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setPeers((await fetchBgpPeers()).data.peers ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const established = peers.filter((p) => p.state === 'Established').length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-teal-500 to-teal-700 flex items-center justify-center shadow-lg shadow-teal-500/20"><Route className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">BGP Peering</h1></div>
          <p className="text-sm text-slate-400 mt-1">BGP peer status and route advertisements</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Peers</div><div className="text-2xl font-bold text-white">{peers.length}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Established</div><div className="text-2xl font-bold text-green-400">{established}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Prefixes Advertised</div><div className="text-2xl font-bold text-white">{peers.reduce((a, p) => a + p.prefixes_advertised, 0)}</div></div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {peers.map((p) => (
          <div key={p.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                {p.state === 'Established' ? <Wifi className="w-5 h-5 text-green-400" /> : <WifiOff className="w-5 h-5 text-red-400" />}
                <div><div className="font-semibold text-white">{p.name}</div><div className="text-xs text-slate-400 font-mono">{p.peer_address}</div></div>
              </div>
              <span className={`px-2 py-0.5 rounded-full text-xs border ${STATE_BADGE[p.state] ?? ''}`}>{p.state}</span>
            </div>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div><span className="text-slate-400">Peer ASN: </span><span className="font-mono text-white">{p.peer_asn}</span></div>
              <div><span className="text-slate-400">Local ASN: </span><span className="font-mono text-white">{p.local_asn}</span></div>
              <div><span className="text-slate-400">Uptime: </span><span className="text-white">{p.uptime}</span></div>
              <div><span className="text-slate-400">Rx Prefixes: </span><span className="text-white">{p.prefixes_received}</span></div>
              <div><span className="text-slate-400">Tx Prefixes: </span><span className="text-white">{p.prefixes_advertised}</span></div>
              <div><span className="text-slate-400">Messages: </span><span className="text-white">{(p.messages_received ?? 0).toLocaleString()} / {(p.messages_sent ?? 0).toLocaleString()}</span></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default BgpPeering;
