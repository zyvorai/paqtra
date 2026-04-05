import React, { useState, useCallback, useMemo } from 'react';
import { Database } from 'lucide-react';
import { fetchEbpfConntrack } from '../../services/api';
import { isAxiosError } from 'axios';
import { formatCount, formatBytes } from '../../utils/formatters';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

interface ConntrackEntry {
  src_ip: string;
  src_port: number;
  dst_ip: string;
  dst_port: number;
  protocol: number;
  rx_packets: number;
  tx_packets: number;
  bytes: number;
  [key: string]: unknown;
}

const PROTO_MAP: Record<number, string> = { 1: 'ICMP', 6: 'TCP', 17: 'UDP' };

type SortField = 'src_ip' | 'src_port' | 'dst_ip' | 'dst_port' | 'protocol' | 'rx_packets' | 'tx_packets' | 'bytes';

const ConntrackViewer: React.FC = () => {
  usePageTitle('Conntrack Table');
  const [entries, setEntries] = useState<ConntrackEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);
  const [search, setSearch] = useState('');
  const [sortField, setSortField] = useState<SortField>('bytes');
  const [sortAsc, setSortAsc] = useState(false);

  const loadData = useCallback(async () => {
    setError(null);
    try {
      const res = await fetchEbpfConntrack();
      setEntries((res.data.entries ?? []) as ConntrackEntry[]);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load conntrack data');
    }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(loadData, 30000, autoRefreshOn);

  const filtered = useMemo(() => {
    let result = entries;
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((e) => e.src_ip?.toLowerCase().includes(q) || e.dst_ip?.toLowerCase().includes(q));
    }
    result = [...result].sort((a, b) => {
      const av = a[sortField] ?? 0;
      const bv = b[sortField] ?? 0;
      if (typeof av === 'string' && typeof bv === 'string') return sortAsc ? av.localeCompare(bv) : bv.localeCompare(av);
      return sortAsc ? (av as number) - (bv as number) : (bv as number) - (av as number);
    });
    return result;
  }, [entries, search, sortField, sortAsc]);

  const tcpCount = entries.filter((e) => e.protocol === 6).length;
  const udpCount = entries.filter((e) => e.protocol === 17).length;
  const establishedCount = entries.filter((e) => (e as Record<string, unknown>).state === 'established' || e.protocol === 6).length;

  const handleSort = (field: SortField) => {
    if (sortField === field) setSortAsc(!sortAsc);
    else { setSortField(field); setSortAsc(false); }
  };

  const sortIcon = (field: SortField) => sortField === field ? (sortAsc ? ' \u25B2' : ' \u25BC') : '';

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500 to-cyan-700 flex items-center justify-center shadow-lg shadow-cyan-500/20">
              <Database className="w-5 h-5 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-white">Conntrack Table</h1>
          </div>
          <p className="text-sm text-slate-400 mt-1">Live kernel connection tracking entries</p>
        </div>
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Entries</div>
          <div className="text-2xl font-bold text-blue-400">{formatCount(entries.length)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">TCP</div>
          <div className="text-2xl font-bold text-green-400">{formatCount(tcpCount)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">UDP</div>
          <div className="text-2xl font-bold text-purple-400">{formatCount(udpCount)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Established</div>
          <div className="text-2xl font-bold text-orange-400">{formatCount(establishedCount)}</div>
        </div>
      </div>

      <div className="mb-4">
        <input
          type="text"
          placeholder="Filter by IP address..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full max-w-sm px-4 py-2 rounded-lg bg-slate-800 border border-slate-700 text-white text-sm placeholder-slate-500 focus:outline-none focus:border-cyan-500"
        />
      </div>

      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        {filtered.length > 0 && (
          <div className="px-4 py-3 border-b border-slate-700/50 flex justify-end">
            <ExportButton data={filtered as unknown as Record<string, unknown>[]} filename="conntrack-entries" />
          </div>
        )}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-slate-900/50">
              <tr className="text-left text-slate-400">
                <th className="px-4 py-3 font-medium cursor-pointer select-none" onClick={() => handleSort('src_ip')}>Source IP{sortIcon('src_ip')}</th>
                <th className="px-4 py-3 font-medium cursor-pointer select-none" onClick={() => handleSort('src_port')}>Src Port{sortIcon('src_port')}</th>
                <th className="px-4 py-3 font-medium cursor-pointer select-none" onClick={() => handleSort('dst_ip')}>Dest IP{sortIcon('dst_ip')}</th>
                <th className="px-4 py-3 font-medium cursor-pointer select-none" onClick={() => handleSort('dst_port')}>Dst Port{sortIcon('dst_port')}</th>
                <th className="px-4 py-3 font-medium cursor-pointer select-none" onClick={() => handleSort('protocol')}>Protocol{sortIcon('protocol')}</th>
                <th className="px-4 py-3 font-medium text-right cursor-pointer select-none" onClick={() => handleSort('rx_packets')}>RX Packets{sortIcon('rx_packets')}</th>
                <th className="px-4 py-3 font-medium text-right cursor-pointer select-none" onClick={() => handleSort('tx_packets')}>TX Packets{sortIcon('tx_packets')}</th>
                <th className="px-4 py-3 font-medium text-right cursor-pointer select-none" onClick={() => handleSort('bytes')}>Total Bytes{sortIcon('bytes')}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/50">
              {filtered.map((e, i) => (
                <tr key={i} className="table-row-hover">
                  <td className="px-4 py-3 text-white font-mono text-xs">{e.src_ip}</td>
                  <td className="px-4 py-3 text-slate-300">{e.src_port}</td>
                  <td className="px-4 py-3 text-white font-mono text-xs">{e.dst_ip}</td>
                  <td className="px-4 py-3 text-slate-300">{e.dst_port}</td>
                  <td className="px-4 py-3">
                    <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                      e.protocol === 6 ? 'bg-green-500/20 text-green-400' :
                      e.protocol === 17 ? 'bg-purple-500/20 text-purple-400' :
                      e.protocol === 1 ? 'bg-yellow-500/20 text-yellow-400' :
                      'bg-slate-500/20 text-slate-400'
                    }`}>{PROTO_MAP[e.protocol] ?? `Proto ${e.protocol}`}</span>
                  </td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(e.rx_packets)}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(e.tx_packets)}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatBytes(e.bytes)}</td>
                </tr>
              ))}
              {filtered.length === 0 && !loading && (
                <tr><td colSpan={8} className="px-4 py-8 text-center text-slate-400">No conntrack entries found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default ConntrackViewer;
