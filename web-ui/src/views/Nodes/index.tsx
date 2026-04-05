import React, { useState, useCallback } from 'react';
import { Server, Loader2, Cpu, MemoryStick, Box } from 'lucide-react';
import { fetchNodes, K8sNode } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const Nodes: React.FC = () => {
  usePageTitle('Nodes');
  const [nodes, setNodes] = useState<K8sNode[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setNodes((await fetchNodes()).data.nodes ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch nodes'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const totalCpu = nodes.reduce((a, n) => a + n.cpu_capacity, 0);
  const totalMem = nodes.reduce((a, n) => a + n.memory_capacity_gb, 0);
  const totalPods = nodes.reduce((a, n) => a + (n.pods_count ?? n.pods), 0);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20"><Server className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Nodes</h1></div>
          <p className="text-sm text-slate-400 mt-1">Kubernetes cluster node overview</p>
        </div>
        <div className="flex items-center gap-2">
          <ExportButton data={nodes as Record<string, unknown>[]} filename="nodes" />
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
            autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Cluster summary */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Nodes</div>
          <div className="text-2xl font-bold text-white">{nodes.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow-purple transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total CPU</div>
          <div className="text-2xl font-bold text-white">{totalCpu} cores</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Memory</div>
          <div className="text-2xl font-bold text-white">{totalMem.toFixed(0)} GB</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Pods</div>
          <div className="text-2xl font-bold text-white">{totalPods}</div>
        </div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Node cards */}
      {!loading && nodes.length === 0 && (
        <div className="text-center py-12 text-slate-400">No nodes found</div>
      )}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {nodes.map((node) => {
          const cpuPct = (node.cpu_usage / node.cpu_capacity) * 100;
          const memPct = (node.memory_usage_gb / node.memory_capacity_gb) * 100;
          const podPct = ((node.pods_count ?? node.pods) / ((node.pods_capacity ?? node.cpu_capacity) || 1)) * 100;
          return (
            <div key={node.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
              <div className="flex items-center justify-between mb-4">
                <div>
                  <div className="font-semibold text-white text-lg">{node.name}</div>
                  <div className="flex gap-1 mt-1">
                    {node.roles.map((r) => (
                      <span key={r} className="px-2 py-0.5 rounded bg-blue-500/15 text-blue-400 text-xs">{r}</span>
                    ))}
                  </div>
                </div>
                <span className={`px-2 py-0.5 rounded-full text-xs border ${node.status === 'Ready' ? 'bg-green-500/15 text-green-400 border-green-500/30' : 'bg-red-500/15 text-red-400 border-red-500/30'}`}>
                  {node.status}
                </span>
              </div>

              {/* Resource bars */}
              <div className="space-y-3">
                <ResourceBar icon={<Cpu className="w-4 h-4" />} label="CPU" used={node.cpu_usage} total={node.cpu_capacity} unit="cores" pct={cpuPct} />
                <ResourceBar icon={<MemoryStick className="w-4 h-4" />} label="Memory" used={node.memory_usage_gb} total={node.memory_capacity_gb} unit="GB" pct={memPct} />
                <ResourceBar icon={<Box className="w-4 h-4" />} label="Pods" used={node.pods_count ?? node.pods} total={node.pods_capacity ?? 0} unit="" pct={podPct} />
              </div>

              {/* Info */}
              <div className="grid grid-cols-2 gap-2 mt-4 pt-3 border-t border-slate-700/50 text-xs">
                <div><span className="text-slate-400">Version: </span><span className="text-white font-mono">{node.version}</span></div>
                <div><span className="text-slate-400">Kernel: </span><span className="text-white font-mono">{node.kernel}</span></div>
                <div><span className="text-slate-400">OS: </span><span className="text-white">{node.os}</span></div>
                <div><span className="text-slate-400">Age: </span><span className="text-white">{node.age}</span></div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

function ResourceBar({ icon, label, used, total, unit, pct }: { icon: React.ReactNode; label: string; used: number; total: number; unit: string; pct: number }) {
  const color = pct >= 90 ? 'bg-red-400' : pct >= 70 ? 'bg-yellow-400' : 'bg-green-400';
  const textColor = pct >= 90 ? 'text-red-400' : pct >= 70 ? 'text-yellow-400' : 'text-green-400';
  return (
    <div>
      <div className="flex items-center justify-between text-sm mb-1">
        <div className="flex items-center gap-2 text-slate-400">{icon} {label}</div>
        <span className={`font-medium ${textColor}`}>{typeof used === 'number' && used % 1 !== 0 ? used.toFixed(1) : used}{unit ? ` ${unit}` : ''} / {typeof total === 'number' && total % 1 !== 0 ? total.toFixed(0) : total}{unit ? ` ${unit}` : ''}</span>
      </div>
      <div className="w-full h-2 rounded-full bg-slate-700 overflow-hidden">
        <div className={`h-full rounded-full ${color} transition-all`} style={{ width: `${Math.min(pct, 100)}%` }} />
      </div>
    </div>
  );
}

export default Nodes;
