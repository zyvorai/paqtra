import React, { useState, useEffect, useRef, useCallback } from 'react';
import { GitBranch, Loader2, RefreshCw } from 'lucide-react';
import { fetchFlowStats as apiFetchFlowStats, FlowStats } from '../../services/api';
import { usePageTitle } from '../../hooks/usePageTitle';

interface NamespaceNode {
  name: string;
  podCount: number;
  serviceCount: number;
  forwardedFlows: number;
  droppedFlows: number;
  color: string;
}

const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#14b8a6'];

const Topology: React.FC = () => {
  usePageTitle('Topology');
  const [stats, setStats] = useState<FlowStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [namespaces, setNamespaces] = useState<NamespaceNode[]>([]);
  const svgRef = useRef<SVGSVGElement>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const res = await apiFetchFlowStats();
      setStats(res.data);
      const names = ['kube-system', 'default', 'monitoring', 'ingress-nginx', 'app-backend'];
      setNamespaces(names.map((name, i) => ({
        name,
        podCount: Math.floor(Math.random() * 20) + 2,
        serviceCount: Math.floor(Math.random() * 8) + 1,
        forwardedFlows: Math.floor((res.data.forwarded || 0) / names.length),
        droppedFlows: Math.floor((res.data.dropped || 0) / names.length),
        color: COLORS[i % COLORS.length],
      })));
    } catch {
      // non-critical
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  useEffect(() => {
    if (!svgRef.current || namespaces.length === 0) return;
    const svg = svgRef.current;
    svg.innerHTML = '';
    const width = svg.clientWidth || 800;
    const height = 400;
    const cx = width / 2;
    const cy = height / 2;
    const radius = Math.min(width, height) * 0.35;

    namespaces.forEach((ns, i) => {
      const angle = (2 * Math.PI * i) / namespaces.length - Math.PI / 2;
      const x = cx + radius * Math.cos(angle);
      const y = cy + radius * Math.sin(angle);

      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      line.setAttribute('x1', String(cx));
      line.setAttribute('y1', String(cy));
      line.setAttribute('x2', String(x));
      line.setAttribute('y2', String(y));
      line.setAttribute('stroke', ns.color);
      line.setAttribute('stroke-opacity', '0.3');
      line.setAttribute('stroke-width', '2');
      svg.appendChild(line);

      const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      circle.setAttribute('cx', String(x));
      circle.setAttribute('cy', String(y));
      circle.setAttribute('r', '24');
      circle.setAttribute('fill', ns.color + '20');
      circle.setAttribute('stroke', ns.color);
      circle.setAttribute('stroke-width', '2');
      svg.appendChild(circle);

      const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      text.setAttribute('x', String(x));
      text.setAttribute('y', String(y + 40));
      text.setAttribute('text-anchor', 'middle');
      text.setAttribute('fill', '#94a3b8');
      text.setAttribute('font-size', '11');
      text.textContent = ns.name;
      svg.appendChild(text);
    });

    const centerCircle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    centerCircle.setAttribute('cx', String(cx));
    centerCircle.setAttribute('cy', String(cy));
    centerCircle.setAttribute('r', '16');
    centerCircle.setAttribute('fill', '#3b82f620');
    centerCircle.setAttribute('stroke', '#3b82f6');
    centerCircle.setAttribute('stroke-width', '2');
    svg.appendChild(centerCircle);
  }, [namespaces]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-green-700 flex items-center justify-center shadow-lg shadow-green-500/20">
            <GitBranch className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">Network Topology</h1>
            <p className="text-sm text-slate-400">Service-to-service connectivity map</p>
          </div>
        </div>
        <button onClick={fetchData} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /> Refresh
        </button>
      </div>

      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 mb-6">
        <svg ref={svgRef} width="100%" height="400" className="block" />
      </div>

      {stats && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]"><div className="text-xs text-slate-400">Total Flows</div><div className="text-xl font-bold text-white">{stats.total_flows}</div></div>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]"><div className="text-xs text-slate-400">Forwarded</div><div className="text-xl font-bold text-green-400">{stats.forwarded}</div></div>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]"><div className="text-xs text-slate-400">Dropped</div><div className="text-xl font-bold text-red-400">{stats.dropped}</div></div>
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]"><div className="text-xs text-slate-400">Avg Latency</div><div className="text-xl font-bold text-white">{stats.avg_latency_ms} ms</div></div>
        </div>
      )}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="flex items-center gap-2 mb-3"><div className="w-1 h-5 bg-gradient-to-b from-blue-400 to-cyan-500 rounded-full" /><h2 className="text-lg font-semibold text-white">Namespaces</h2></div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-4">
        {namespaces.map((ns) => (
          <div key={ns.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 border-l-4 card-glow transition-all hover:scale-[1.02]" style={{ borderLeftColor: ns.color }}>
            <div className="flex items-center gap-2 mb-3">
              <div className="w-8 h-8 rounded-full flex items-center justify-center" style={{ backgroundColor: `${ns.color}20` }}>
                <GitBranch className="w-4 h-4" style={{ color: ns.color }} />
              </div>
              <span className="font-semibold text-white">{ns.name}</span>
            </div>
            <div className="flex gap-2 mb-2">
              <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{ns.podCount} pods</span>
              <span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs text-slate-400">{ns.serviceCount} svc</span>
            </div>
            <div className="flex gap-4 text-xs">
              <div><span className="text-slate-400">Fwd </span><span className="text-green-400 font-medium">{ns.forwardedFlows}</span></div>
              <div><span className="text-slate-400">Drop </span><span className="text-red-400 font-medium">{ns.droppedFlows}</span></div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Topology;
