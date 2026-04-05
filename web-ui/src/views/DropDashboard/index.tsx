import React, { useState, useCallback, useMemo } from 'react';
import { AlertTriangle } from 'lucide-react';
import { fetchEbpfDrops } from '../../services/api';
import { isAxiosError } from 'axios';
import { formatCount, formatBytes } from '../../utils/formatters';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';

interface DropEntry {
  reason: string;
  reason_code: number;
  count: number;
  bytes: number;
  [key: string]: unknown;
}

const DROP_COLORS: Record<string, string> = {
  PolicyDenied: '#ef4444',
  NoRoute: '#f97316',
  InvalidPacket: '#eab308',
  CTMapInsertion: '#8b5cf6',
  FragNeeded: '#3b82f6',
  Unsupported: '#6b7280',
};

const DROP_BG_COLORS: Record<string, string> = {
  PolicyDenied: 'bg-red-500/20 text-red-400',
  NoRoute: 'bg-orange-500/20 text-orange-400',
  InvalidPacket: 'bg-yellow-500/20 text-yellow-400',
  CTMapInsertion: 'bg-purple-500/20 text-purple-400',
  FragNeeded: 'bg-blue-500/20 text-blue-400',
};

function getDropColor(reason: string): string {
  return DROP_COLORS[reason] ?? '#6b7280';
}

function getDropBg(reason: string): string {
  return DROP_BG_COLORS[reason] ?? 'bg-slate-500/20 text-slate-400';
}

const DropDashboard: React.FC = () => {
  usePageTitle('Drop Analytics');
  const [drops, setDrops] = useState<DropEntry[]>([]);
  const [totalDrops, setTotalDrops] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const loadData = useCallback(async () => {
    setError(null);
    try {
      const res = await fetchEbpfDrops();
      const rawDrops = (res.data.drops ?? []) as DropEntry[];
      setDrops(rawDrops.sort((a, b) => b.count - a.count));
      setTotalDrops(res.data.total_drops ?? rawDrops.reduce((s, d) => s + d.count, 0));
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load drop data');
    }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(loadData, 30000, autoRefreshOn);

  const policyDenied = useMemo(() => drops.filter((d) => d.reason === 'PolicyDenied').reduce((s, d) => s + d.count, 0), [drops]);
  const noRoute = useMemo(() => drops.filter((d) => d.reason === 'NoRoute').reduce((s, d) => s + d.count, 0), [drops]);
  const invalidPacket = useMemo(() => drops.filter((d) => d.reason === 'InvalidPacket').reduce((s, d) => s + d.count, 0), [drops]);

  const chartData = useMemo(() => drops.slice(0, 10).map((d) => ({ name: d.reason, count: d.count })), [drops]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
              <AlertTriangle className="w-5 h-5 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-white">Drop Analytics</h1>
          </div>
          <p className="text-sm text-slate-400 mt-1">Packet drop reasons from kernel eBPF metrics</p>
        </div>
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Drops</div>
          <div className="text-2xl font-bold text-red-400">{formatCount(totalDrops)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Policy Denied</div>
          <div className="text-2xl font-bold text-orange-400">{formatCount(policyDenied)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-yellow card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">No Route</div>
          <div className="text-2xl font-bold text-yellow-400">{formatCount(noRoute)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Invalid Packet</div>
          <div className="text-2xl font-bold text-purple-400">{formatCount(invalidPacket)}</div>
        </div>
      </div>

      {/* Bar Chart */}
      {chartData.length > 0 && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6 mb-6">
          <h2 className="text-sm font-medium text-slate-400 mb-4">Drops by Reason</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData} layout="vertical" margin={{ left: 120, right: 20, top: 5, bottom: 5 }}>
              <XAxis type="number" tick={{ fill: '#94a3b8', fontSize: 12 }} />
              <YAxis type="category" dataKey="name" tick={{ fill: '#94a3b8', fontSize: 12 }} width={110} />
              <Tooltip
                contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #334155', borderRadius: '8px', color: '#e2e8f0' }}
                formatter={(value: number) => [formatCount(value), 'Count']}
              />
              <Bar dataKey="count" radius={[0, 4, 4, 0]}>
                {chartData.map((entry, index) => (
                  <Cell key={index} fill={getDropColor(entry.name)} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        {drops.length > 0 && (
          <div className="px-4 py-3 border-b border-slate-700/50 flex justify-end">
            <ExportButton data={drops as unknown as Record<string, unknown>[]} filename="drop-analytics" />
          </div>
        )}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-slate-900/50">
              <tr className="text-left text-slate-400">
                <th className="px-4 py-3 font-medium">Reason</th>
                <th className="px-4 py-3 font-medium">Reason Code</th>
                <th className="px-4 py-3 font-medium text-right">Count</th>
                <th className="px-4 py-3 font-medium text-right">Bytes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/50">
              {drops.map((d, i) => (
                <tr key={i} className="table-row-hover">
                  <td className="px-4 py-3">
                    <span className={`px-2 py-0.5 rounded text-xs font-medium ${getDropBg(d.reason)}`}>{d.reason}</span>
                  </td>
                  <td className="px-4 py-3 text-slate-300 font-mono text-xs">{d.reason_code}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(d.count)}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatBytes(d.bytes)}</td>
                </tr>
              ))}
              {drops.length === 0 && !loading && (
                <tr><td colSpan={4} className="px-4 py-8 text-center text-slate-400">No drop data found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};

export default DropDashboard;
