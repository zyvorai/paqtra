import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Map, RefreshCw, Loader2 } from 'lucide-react';
import { select } from 'd3-selection';
import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide, type SimulationNodeDatum, type SimulationLinkDatum } from 'd3-force';
import { drag as d3Drag } from 'd3-drag';
import { fetchServiceMap, ServiceNode, ServiceEdge } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATUS_COLOR: Record<string, string> = { healthy: '#22c55e', degraded: '#eab308', unhealthy: '#ef4444' };

const ServiceMapView: React.FC = () => {
  usePageTitle('Service Map');
  const [nodes, setNodes] = useState<ServiceNode[]>([]);
  const [edges, setEdges] = useState<ServiceEdge[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { const res = await fetchServiceMap(); setNodes(res.data.nodes ?? []); setEdges(res.data.edges ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  // D3 force simulation
  useEffect(() => {
    if (!svgRef.current || nodes.length === 0) return;

    const width = containerRef.current?.clientWidth ?? 800;
    const height = 500;
    const svg = select(svgRef.current);
    svg.selectAll('*').remove();
    svg.attr('viewBox', `0 0 ${width} ${height}`);

    const defs = svg.append('defs');
    defs.append('marker').attr('id', 'arrowhead').attr('viewBox', '0 -5 10 10').attr('refX', 30).attr('refY', 0)
      .attr('markerWidth', 6).attr('markerHeight', 6).attr('orient', 'auto')
      .append('path').attr('d', 'M0,-5L10,0L0,5').attr('fill', '#64748b');

    interface SimNode extends SimulationNodeDatum { id: string; data: ServiceNode }
    interface SimLink extends SimulationLinkDatum<SimNode> { data: ServiceEdge }

    const simNodes: SimNode[] = nodes.map((n) => ({ id: n.name, data: n }));
    const simLinks: SimLink[] = edges.map((e) => ({ source: e.source, target: e.target, data: e }));

    const simulation = forceSimulation<SimNode>(simNodes)
      .force('link', forceLink<SimNode, SimLink>(simLinks).id((d) => d.id).distance(150))
      .force('charge', forceManyBody().strength(-400))
      .force('center', forceCenter(width / 2, height / 2))
      .force('collision', forceCollide().radius(50));

    const linkGroup = svg.append('g');
    const link = linkGroup.selectAll('line').data(simLinks).enter().append('line')
      .attr('stroke', '#334155').attr('stroke-width', 2).attr('marker-end', 'url(#arrowhead)');

    const linkLabel = linkGroup.selectAll('text').data(simLinks).enter().append('text')
      .attr('fill', '#64748b').attr('font-size', '10').attr('text-anchor', 'middle')
      .text((d) => `${d.data.protocol} ${d.data.request_rate}/s`);

    const nodeGroup = svg.append('g');
    const node = nodeGroup.selectAll('g').data(simNodes).enter().append('g')
      .call(d3Drag<SVGGElement, SimNode>()
        .on('start', (event, d) => { if (!event.active) simulation.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; })
        .on('drag', (event, d) => { d.fx = event.x; d.fy = event.y; })
        .on('end', (event, d) => { if (!event.active) simulation.alphaTarget(0); d.fx = null; d.fy = null; })
      );

    node.append('circle').attr('r', 24).attr('fill', (d) => `${STATUS_COLOR[d.data.status ?? ''] ?? '#64748b'}20`)
      .attr('stroke', (d) => STATUS_COLOR[d.data.status ?? ''] ?? '#64748b').attr('stroke-width', 2);

    node.append('text').attr('text-anchor', 'middle').attr('dy', 4).attr('fill', '#e2e8f0')
      .attr('font-size', '11').attr('font-weight', '600').text((d) => d.data.name.slice(0, 8));

    node.append('text').attr('text-anchor', 'middle').attr('dy', 42).attr('fill', '#94a3b8')
      .attr('font-size', '9').text((d) => `${d.data.pods} pods | ${d.data.request_rate}/s`);

    node.append('title').text((d) => `${d.data.name}\nStatus: ${d.data.status}\nPods: ${d.data.pods}\nRPS: ${d.data.request_rate}\nErrors: ${d.data.error_rate}%`);

    simulation.on('tick', () => {
      link.attr('x1', (d) => (d.source as SimNode).x!).attr('y1', (d) => (d.source as SimNode).y!)
        .attr('x2', (d) => (d.target as SimNode).x!).attr('y2', (d) => (d.target as SimNode).y!);
      linkLabel.attr('x', (d) => ((d.source as SimNode).x! + (d.target as SimNode).x!) / 2)
        .attr('y', (d) => ((d.source as SimNode).y! + (d.target as SimNode).y!) / 2 - 8);
      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
    });

    return () => { simulation.stop(); };
  }, [nodes, edges]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-emerald-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-emerald-500/20"><Map className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Service Map</h1></div>
          <p className="text-sm text-slate-400 mt-1">Interactive force-directed service topology (drag nodes to rearrange)</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div ref={containerRef} className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden mb-6">
        <svg ref={svgRef} width="100%" height="500" className="block" />
      </div>

      {/* Legend */}
      <div className="flex items-center gap-6 text-xs text-slate-400 mb-6">
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-green-500" /> Healthy</span>
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-yellow-500" /> Degraded</span>
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-red-500" /> Unhealthy</span>
        <span className="text-slate-400">Drag nodes to rearrange</span>
      </div>

      {/* Service table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <table className="w-full text-sm">
          <thead><tr className="border-b border-slate-700/50 bg-slate-900/50">
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Service</th>
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Namespace</th>
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Type</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Pods</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">RPS</th>
            <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Error %</th>
            <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Status</th>
          </tr></thead>
          <tbody>{nodes.map((n) => (
            <tr key={n.name} className="border-b border-slate-700/30 table-row-hover">
              <td className="px-4 py-2.5 font-medium text-white">{n.name}</td>
              <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{n.namespace}</span></td>
              <td className="px-4 py-2.5 text-slate-400">{n.type}</td>
              <td className="px-4 py-2.5 text-right text-white">{n.pods}</td>
              <td className="px-4 py-2.5 text-right text-white">{n.request_rate}</td>
              <td className="px-4 py-2.5 text-right"><span className={(n.error_rate ?? 0) > 1 ? 'text-red-400' : 'text-green-400'}>{(n.error_rate ?? 0)}%</span></td>
              <td className="px-4 py-2.5"><span className="flex items-center gap-1.5"><span className={`w-2 h-2 rounded-full`} style={{ backgroundColor: STATUS_COLOR[n.status ?? ''] }} />{n.status}</span></td>
            </tr>
          ))}</tbody>
        </table>
      </div>
    </div>
  );
};

export default ServiceMapView;
