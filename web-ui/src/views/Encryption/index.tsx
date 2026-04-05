import React, { useState, useCallback } from 'react';
import { Lock, Loader2, ShieldCheck, ShieldOff, Key, ArrowUpDown } from 'lucide-react';
import { fetchEncryptionStatus, EncryptionStatus } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

function formatBytes(b: number): string { return b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${(b / 1e3).toFixed(1)} KB`; }

const Encryption: React.FC = () => {
  usePageTitle('Encryption');
  const [status, setStatus] = useState<EncryptionStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setStatus((await fetchEncryptionStatus()).data); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const pct = status ? (status.nodes_encrypted / status.nodes_total) * 100 : 0;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 flex items-center justify-center shadow-lg shadow-slate-500/20"><Lock className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Encryption Status</h1></div>
          <p className="text-sm text-slate-400 mt-1">Node-to-node encryption monitoring</p>
        </div>
        <div className="flex items-center gap-3">
          {status && (status.interfaces ?? []).length > 0 && <ExportButton data={(status.interfaces ?? []) as unknown as Record<string, unknown>[]} filename="encryption-interfaces" />}
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && !status && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {!loading && !status && !error && (
        <div className="text-center py-12 text-slate-400">No encryption data available.</div>
      )}

      {status && (
        <>
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
            <div className={`rounded-xl border border-slate-700/50 p-4 ${status.enabled ? 'stat-card-green' : 'stat-card-red'}`}>
              <div className="flex items-center gap-2 mb-2">{status.enabled ? <ShieldCheck className="w-5 h-5 text-green-400" /> : <ShieldOff className="w-5 h-5 text-red-400" />}<span className="text-xs text-slate-400">Status</span></div>
              <div className={`text-2xl font-bold ${status.enabled ? 'text-green-400' : 'text-red-400'}`}>{status.enabled ? 'Enabled' : 'Disabled'}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
              <div className="text-xs text-slate-400 mb-2">Type</div>
              <div className="text-2xl font-bold text-white">{status.type}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]">
              <div className="text-xs text-slate-400 mb-2">Nodes Encrypted</div>
              <div className="text-2xl font-bold text-white">{status.nodes_encrypted} / {status.nodes_total}</div>
              <div className="w-full h-1.5 rounded-full bg-black/20 mt-2 overflow-hidden">
                <div className={`h-full rounded-full ${pct >= 100 ? 'bg-green-400' : pct > 0 ? 'bg-yellow-400' : 'bg-red-400'}`} style={{ width: `${pct}%` }} />
              </div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
              <div className="flex items-center gap-1 text-xs text-slate-400 mb-2"><Key className="w-3 h-3" /> Key Rotation</div>
              <div className="text-lg font-bold text-white">{status.key_rotation_at ? new Date(status.key_rotation_at).toLocaleDateString() : '-'}</div>
            </div>
          </div>

          <div className="flex items-center gap-2 mb-3"><div className="w-1 h-5 bg-gradient-to-b from-slate-400 to-blue-500 rounded-full" /><h2 className="text-lg font-semibold text-white">Tunnel Interfaces</h2></div>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
            <table className="w-full text-sm">
              <thead><tr className="border-b border-slate-700/50 bg-slate-900/50">
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Interface</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Peer</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Endpoint</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Last Handshake</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">TX</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">RX</th>
              </tr></thead>
              <tbody>
                {(status.interfaces ?? []).length === 0 && (
                  <tr><td colSpan={6} className="px-4 py-8 text-center text-slate-400">No tunnel interfaces found.</td></tr>
                )}
                {(status.interfaces ?? []).map((iface) => (
                <tr key={iface.name ?? iface.interface ?? '-'} className="border-b border-slate-700/30 table-row-hover">
                  <td className="px-4 py-2.5 font-mono text-white">{iface.name ?? iface.interface ?? '-'}</td>
                  <td className="px-4 py-2.5 font-mono text-white">{iface.peer ?? iface.public_key?.slice(0, 20) ?? '-'}</td>
                  <td className="px-4 py-2.5 text-slate-400">{iface.endpoint ?? iface.node ?? '-'}</td>
                  <td className="px-4 py-2.5 text-slate-400">{iface.latest_handshake ?? '-'}</td>
                  <td className="px-4 py-2.5 text-right text-white flex items-center justify-end gap-1"><ArrowUpDown className="w-3 h-3 text-slate-400" />{formatBytes(iface.tx_bytes ?? iface.stats?.tx_bytes ?? 0)}</td>
                  <td className="px-4 py-2.5 text-right text-white">{formatBytes(iface.rx_bytes ?? iface.stats?.rx_bytes ?? 0)}</td>
                </tr>
              ))}</tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
};

export default Encryption;
