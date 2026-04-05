import React, { useState, useCallback } from 'react';
import { Gauge, Loader2, ArrowUp, ArrowDown } from 'lucide-react';
import { fetchBandwidthData, BandwidthEntry } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

function formatBytes(b: number): string {
  if (b >= 1e9) return `${(b / 1e9).toFixed(1)} GB`;
  if (b >= 1e6) return `${(b / 1e6).toFixed(1)} MB`;
  return `${(b / 1e3).toFixed(1)} KB`;
}

function rateColor(rate: number, limit: number | null): string {
  if (!limit) return 'text-white';
  const pct = (rate / limit) * 100;
  return pct >= 90 ? 'text-red-400' : pct >= 70 ? 'text-yellow-400' : 'text-green-400';
}

const Bandwidth: React.FC = () => {
  usePageTitle('Bandwidth');
  const [entries, setEntries] = useState<BandwidthEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setEntries((await fetchBandwidthData()).data.entries ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const totalTx = entries.reduce((a, e) => a + e.egress_rate_mbps, 0);
  const totalRx = entries.reduce((a, e) => a + e.ingress_rate_mbps, 0);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20"><Gauge className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Bandwidth Manager</h1></div>
          <p className="text-sm text-slate-400 mt-1">Per-pod bandwidth monitoring and rate limiting</p>
        </div>
        <div className="flex items-center gap-2">
          <ExportButton data={entries as Record<string, unknown>[]} filename="bandwidth" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Pods Monitored</div><div className="text-2xl font-bold text-white">{entries.length}</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="flex items-center gap-1 text-xs text-slate-400 mb-1"><ArrowUp className="w-3 h-3" /> Total Egress</div><div className="text-2xl font-bold text-white">{totalTx.toFixed(0)} Mbps</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]"><div className="flex items-center gap-1 text-xs text-slate-400 mb-1"><ArrowDown className="w-3 h-3" /> Total Ingress</div><div className="text-2xl font-bold text-white">{totalRx.toFixed(0)} Mbps</div></div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Rate Limited</div><div className="text-2xl font-bold text-white">{entries.filter((e) => e.egress_limit_mbps || e.ingress_limit_mbps).length}</div></div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <table className="w-full text-sm">
          <thead><tr className="border-b border-slate-700/50 bg-slate-900/50">
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Pod</th>
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Namespace</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Egress</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Ingress</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Total TX</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Total RX</th>
          </tr></thead>
          <tbody>{!loading && entries.length === 0 && (
            <tr><td colSpan={6} className="px-4 py-12 text-center text-slate-400">No bandwidth data found</td></tr>
          )}{entries.map((e) => (
            <tr key={e.pod} className="border-b border-slate-700/30 table-row-hover">
              <td className="px-4 py-2.5 font-mono text-white">{e.pod}</td>
              <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{e.namespace}</span></td>
              <td className="px-4 py-2.5 text-right">
                <span className={`font-medium ${rateColor(e.egress_rate_mbps, e.egress_limit_mbps)}`}>{e.egress_rate_mbps} Mbps</span>
                {e.egress_limit_mbps && <span className="text-xs text-slate-400 ml-1">/ {e.egress_limit_mbps}</span>}
              </td>
              <td className="px-4 py-2.5 text-right">
                <span className={`font-medium ${rateColor(e.ingress_rate_mbps, e.ingress_limit_mbps)}`}>{e.ingress_rate_mbps} Mbps</span>
                {e.ingress_limit_mbps && <span className="text-xs text-slate-400 ml-1">/ {e.ingress_limit_mbps}</span>}
              </td>
              <td className="px-4 py-2.5 text-right text-slate-400">{formatBytes(e.total_bytes_tx)}</td>
              <td className="px-4 py-2.5 text-right text-slate-400">{formatBytes(e.total_bytes_rx)}</td>
            </tr>
          ))}</tbody>
        </table>
      </div>
    </div>
  );
};

export default Bandwidth;
