import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { Map, Loader2 } from 'lucide-react';
import { select } from 'd3-selection';
import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide, type SimulationNodeDatum, type SimulationLinkDatum } from 'd3-force';
import { drag as d3Drag } from 'd3-drag';
import { fetchServiceMap, ServiceNode, ServiceEdge } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import { useNamespaceStore } from '../../stores/namespaceStore';

const STATUS_COLOR: Record<string, string> = { healthy: '#22c55e', degraded: '#eab308', unhealthy: '#ef4444' };

// ── Verdict-based edge helpers ──────────────────────────────

const VERDICT_GREEN = '#22c55e';
const VERDICT_RED = '#ef4444';
const VERDICT_ORANGE = '#f59e0b';
const VERDICT_NEUTRAL = '#334155';

/** Edge color based on verdict breakdown (forwarded vs dropped). */
function edgeColor(edge: ServiceEdge): string {
  const flowCount = edge.flow_count ?? 0;
  const droppedCount = edge.dropped_count ?? 0;
  if (flowCount === 0) return VERDICT_NEUTRAL;
  if (droppedCount === 0) return VERDICT_GREEN;
  if (droppedCount >= flowCount) return VERDICT_RED;
  return VERDICT_ORANGE;
}

/** Edge thickness scaled by flow_count (range 1.5 - 8). */
function edgeWidth(edge: ServiceEdge, maxFlowCount: number): number {
  const count = edge.flow_count ?? 0;
  if (maxFlowCount === 0) return 2;
  return 1.5 + (count / maxFlowCount) * 6.5;
}

/** Edge opacity scaled by flow_count (range 0.4 - 1.0). */
function edgeOpacity(edge: ServiceEdge, maxFlowCount: number): number {
  const count = edge.flow_count ?? 0;
  if (maxFlowCount === 0) return 0.7;
  return 0.4 + (count / maxFlowCount) * 0.6;
}

const ServiceMapView: React.FC = () => {
  usePageTitle('Service Map');
  const { selectedNamespace } = useNamespaceStore();
  const navigate = useNavigate();
  const [allNodes, setAllNodes] = useState<ServiceNode[]>([]);
  const [allEdges, setAllEdges] = useState<ServiceEdge[]>([]);
  const [error, setError] = useState<string | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(false);

  const fetchData = useCallback(async () => {
    setError(null);
    try { const res = await fetchServiceMap(); setAllNodes(res.data.nodes ?? []); setAllEdges(res.data.edges ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  // Filter nodes and edges by the globally selected namespace
  const nodes = useMemo(() => {
    if (!selectedNamespace) return allNodes;
    return allNodes.filter((n) => n.namespace === selectedNamespace);
  }, [allNodes, selectedNamespace]);

  const edges = useMemo(() => {
    if (!selectedNamespace) return allEdges;
    const nodeNames = new Set(nodes.map((n) => n.name));
    return allEdges.filter((e) => nodeNames.has(e.source) || nodeNames.has(e.target));
  }, [allEdges, selectedNamespace, nodes]);

  // D3 force simulation
  useEffect(() => {
    if (!svgRef.current || nodes.length === 0) return;

    const width = containerRef.current?.clientWidth ?? 800;
    const height = 500;
    const svg = select(svgRef.current);
    svg.selectAll('*').remove();
    svg.attr('viewBox', `0 0 ${width} ${height}`);

    // Compute max flow_count for scaling thickness / opacity
    const maxFlowCount = Math.max(1, ...edges.map((e) => e.flow_count ?? 0));

    // ── Defs: per-color arrowhead markers ───────────────────
    const defs = svg.append('defs');
    const arrowColors: Record<string, string> = {
      green: VERDICT_GREEN,
      red: VERDICT_RED,
      orange: VERDICT_ORANGE,
      gray: '#64748b',
    };
    for (const [name, color] of Object.entries(arrowColors)) {
      defs.append('marker')
        .attr('id', `arrowhead-${name}`)
        .attr('viewBox', '0 -5 10 10')
        .attr('refX', 30).attr('refY', 0)
        .attr('markerWidth', 6).attr('markerHeight', 6)
        .attr('orient', 'auto')
        .append('path').attr('d', 'M0,-5L10,0L0,5').attr('fill', color);
    }

    function arrowMarker(edge: ServiceEdge): string {
      const c = edgeColor(edge);
      if (c === VERDICT_GREEN) return 'url(#arrowhead-green)';
      if (c === VERDICT_RED) return 'url(#arrowhead-red)';
      if (c === VERDICT_ORANGE) return 'url(#arrowhead-orange)';
      return 'url(#arrowhead-gray)';
    }

    interface SimNode extends SimulationNodeDatum { id: string; data: ServiceNode }
    interface SimLink extends SimulationLinkDatum<SimNode> { data: ServiceEdge }

    const simNodes: SimNode[] = nodes.map((n) => ({ id: n.name, data: n }));
    const simLinks: SimLink[] = edges.map((e) => ({ source: e.source, target: e.target, data: e }));

    const simulation = forceSimulation<SimNode>(simNodes)
      .force('link', forceLink<SimNode, SimLink>(simLinks).id((d) => d.id).distance(150))
      .force('charge', forceManyBody().strength(-400))
      .force('center', forceCenter(width / 2, height / 2))
      .force('collision', forceCollide().radius(50));

    // ── Tooltip (attached to body, positioned on hover) ─────
    select('#service-map-tooltip').remove();
    const tooltip = select('body').append('div')
      .attr('id', 'service-map-tooltip')
      .style('position', 'fixed')
      .style('pointer-events', 'none')
      .style('background', '#1e293b')
      .style('border', '1px solid #334155')
      .style('border-radius', '8px')
      .style('padding', '10px 14px')
      .style('font-size', '12px')
      .style('color', '#e2e8f0')
      .style('line-height', '1.6')
      .style('z-index', '9999')
      .style('box-shadow', '0 4px 12px rgba(0,0,0,0.4)')
      .style('display', 'none');

    // ── Edges ───────────────────────────────────────────────
    const linkGroup = svg.append('g');

    // Invisible wider hit-area for hover / click
    const linkHitArea = linkGroup.selectAll<SVGLineElement, SimLink>('line.hit-area')
      .data(simLinks).enter().append('line')
      .attr('class', 'hit-area')
      .attr('stroke', 'transparent')
      .attr('stroke-width', 14)
      .style('cursor', 'pointer')
      .on('mouseenter', (_event: MouseEvent, d: SimLink) => {
        const e = d.data;
        const flowCount = e.flow_count ?? 0;
        const droppedCount = e.dropped_count ?? 0;
        const forwardedCount = flowCount - droppedCount;
        const errRate = flowCount > 0 ? ((droppedCount / flowCount) * 100).toFixed(1) : '0.0';
        tooltip.html(
          `<div style="font-weight:600;margin-bottom:4px">${e.source} &rarr; ${e.target}</div>` +
          `<div style="color:#94a3b8">Protocol: <span style="color:#e2e8f0">${e.protocol}</span></div>` +
          `<div style="color:#94a3b8">Flow count: <span style="color:#e2e8f0">${flowCount}</span></div>` +
          `<div style="color:#94a3b8">Forwarded: <span style="color:${VERDICT_GREEN}">${forwardedCount}</span></div>` +
          `<div style="color:#94a3b8">Dropped: <span style="color:${VERDICT_RED}">${droppedCount}</span></div>` +
          `<div style="color:#94a3b8">Error rate: <span style="color:${parseFloat(errRate) > 0 ? VERDICT_RED : VERDICT_GREEN}">${errRate}%</span></div>`
        ).style('display', 'block');
      })
      .on('mousemove', (event: MouseEvent) => {
        tooltip
          .style('left', `${event.clientX + 12}px`)
          .style('top', `${event.clientY - 10}px`);
      })
      .on('mouseleave', () => {
        tooltip.style('display', 'none');
      })
      .on('click', (_event: MouseEvent, d: SimLink) => {
        const srcNode = simNodes.find((n) => n.id === (typeof d.source === 'string' ? d.source : (d.source as SimNode).id));
        const tgtNode = simNodes.find((n) => n.id === (typeof d.target === 'string' ? d.target : (d.target as SimNode).id));
        const ns = srcNode?.data.namespace ?? '';
        const search = tgtNode?.data.name ?? '';
        navigate(`/flows?namespace=${encodeURIComponent(ns)}&search=${encodeURIComponent(search)}`);
      });

    // Visible edge line — verdict-colored, thickness/opacity scaled
    const link = linkGroup.selectAll<SVGLineElement, SimLink>('line.edge')
      .data(simLinks).enter().append('line')
      .attr('class', 'edge')
      .attr('stroke', (d) => edgeColor(d.data))
      .attr('stroke-width', (d) => edgeWidth(d.data, maxFlowCount))
      .attr('stroke-opacity', (d) => edgeOpacity(d.data, maxFlowCount))
      .attr('marker-end', (d) => arrowMarker(d.data))
      .style('pointer-events', 'none'); // hit-area handles events

    // Edge label: protocol + flow count
    const linkLabel = linkGroup.selectAll<SVGTextElement, SimLink>('text')
      .data(simLinks).enter().append('text')
      .attr('fill', '#64748b')
      .attr('font-size', '10')
      .attr('text-anchor', 'middle')
      .style('pointer-events', 'none')
      .text((d) => {
        const count = d.data.flow_count ?? d.data.request_rate;
        return `${d.data.protocol} (${count})`;
      });

    // ── Nodes ───────────────────────────────────────────────
    const nodeGroup = svg.append('g');
    const node = nodeGroup.selectAll<SVGGElement, SimNode>('g')
      .data(simNodes).enter().append('g')
      .call(d3Drag<SVGGElement, SimNode>()
        .on('start', (event, d) => { if (!event.active) simulation.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; })
        .on('drag', (event, d) => { d.fx = event.x; d.fy = event.y; })
        .on('end', (event, d) => { if (!event.active) simulation.alphaTarget(0); d.fx = null; d.fy = null; })
      );

    node.style('cursor', 'pointer')
      .on('click', (_event: MouseEvent, d: SimNode) => {
        const ns = d.data.namespace;
        const podPrefix = d.data.name;
        navigate(`/flows?namespace=${encodeURIComponent(ns)}&search=${encodeURIComponent(podPrefix)}`);
      });

    node.append('circle').attr('r', 24).attr('fill', (d) => `${STATUS_COLOR[d.data.status ?? ''] ?? '#64748b'}20`)
      .attr('stroke', (d) => STATUS_COLOR[d.data.status ?? ''] ?? '#64748b').attr('stroke-width', 2);

    node.append('text').attr('text-anchor', 'middle').attr('dy', 4).attr('fill', '#e2e8f0')
      .attr('font-size', '11').attr('font-weight', '600').text((d) => d.data.name.slice(0, 8));

    node.append('text').attr('text-anchor', 'middle').attr('dy', 42).attr('fill', '#94a3b8')
      .attr('font-size', '9').text((d) => `${d.data.pods} pods | ${d.data.request_rate}/s`);

    node.append('title').text((d) => `${d.data.name}\nStatus: ${d.data.status}\nPods: ${d.data.pods}\nRPS: ${d.data.request_rate}\nErrors: ${d.data.error_rate}%`);

    // ── In-SVG legend (top-right corner) ────────────────────
    const legendG = svg.append('g').attr('transform', `translate(${width - 200}, 16)`);

    legendG.append('rect')
      .attr('x', -12).attr('y', -8)
      .attr('width', 196).attr('height', 112)
      .attr('rx', 8)
      .attr('fill', '#0f172a').attr('fill-opacity', 0.85)
      .attr('stroke', '#334155').attr('stroke-width', 1);

    legendG.append('text')
      .attr('x', 0).attr('y', 8)
      .attr('fill', '#94a3b8').attr('font-size', '10').attr('font-weight', '600')
      .text('EDGE VERDICT');

    const legendItems = [
      { label: 'All forwarded', color: VERDICT_GREEN },
      { label: 'Mixed (some drops)', color: VERDICT_ORANGE },
      { label: 'All dropped', color: VERDICT_RED },
    ];
    legendItems.forEach((item, i) => {
      const y = 28 + i * 18;
      legendG.append('line')
        .attr('x1', 0).attr('y1', y).attr('x2', 24).attr('y2', y)
        .attr('stroke', item.color).attr('stroke-width', 3);
      legendG.append('text')
        .attr('x', 32).attr('y', y + 4)
        .attr('fill', '#e2e8f0').attr('font-size', '10')
        .text(item.label);
    });

    // Thickness sub-legend
    const thickY = 28 + legendItems.length * 18;
    legendG.append('line')
      .attr('x1', 0).attr('y1', thickY).attr('x2', 20).attr('y2', thickY)
      .attr('stroke', '#64748b').attr('stroke-width', 1.5);
    legendG.append('line')
      .attr('x1', 24).attr('y1', thickY).attr('x2', 44).attr('y2', thickY)
      .attr('stroke', '#64748b').attr('stroke-width', 5);
    legendG.append('text')
      .attr('x', 52).attr('y', thickY + 4)
      .attr('fill', '#94a3b8').attr('font-size', '10')
      .text('= traffic volume');

    // ── Tick ────────────────────────────────────────────────
    simulation.on('tick', () => {
      link
        .attr('x1', (d) => (d.source as SimNode).x!).attr('y1', (d) => (d.source as SimNode).y!)
        .attr('x2', (d) => (d.target as SimNode).x!).attr('y2', (d) => (d.target as SimNode).y!);
      linkHitArea
        .attr('x1', (d) => (d.source as SimNode).x!).attr('y1', (d) => (d.source as SimNode).y!)
        .attr('x2', (d) => (d.target as SimNode).x!).attr('y2', (d) => (d.target as SimNode).y!);
      linkLabel
        .attr('x', (d) => ((d.source as SimNode).x! + (d.target as SimNode).x!) / 2)
        .attr('y', (d) => ((d.source as SimNode).y! + (d.target as SimNode).y!) / 2 - 8);
      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
    });

    return () => {
      simulation.stop();
      tooltip.remove();
    };
  }, [nodes, edges, navigate]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-emerald-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-emerald-500/20"><Map className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Service Map</h1></div>
          <p className="text-sm text-slate-400 mt-1">Interactive force-directed service topology (drag nodes to rearrange)</p>
        </div>
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading}
          autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn(v => !v)} intervalSecs={30} />
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div ref={containerRef} className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden mb-6 relative">
        <svg ref={svgRef} width="100%" height="500" className="block" />
      </div>

      {/* Combined legend: node status + edge verdict */}
      <div className="flex flex-wrap items-center gap-6 text-xs text-slate-400 mb-6">
        <span className="font-semibold text-slate-300 uppercase tracking-wider">Nodes:</span>
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-green-500" /> Healthy</span>
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-yellow-500" /> Degraded</span>
        <span className="flex items-center gap-1.5"><span className="w-3 h-3 rounded-full bg-red-500" /> Unhealthy</span>
        <span className="mx-2 border-l border-slate-600 h-4" />
        <span className="font-semibold text-slate-300 uppercase tracking-wider">Edges:</span>
        <span className="flex items-center gap-1.5"><span className="w-4 h-0.5 rounded" style={{ backgroundColor: VERDICT_GREEN }} /> All forwarded</span>
        <span className="flex items-center gap-1.5"><span className="w-4 h-0.5 rounded" style={{ backgroundColor: VERDICT_ORANGE }} /> Mixed (some drops)</span>
        <span className="flex items-center gap-1.5"><span className="w-4 h-0.5 rounded" style={{ backgroundColor: VERDICT_RED }} /> All dropped</span>
        <span className="text-slate-500">| Line thickness = traffic volume | Click to view flows</span>
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
            <tr key={n.name} className="border-b border-slate-700/30 table-row-hover cursor-pointer"
              onClick={() => navigate(`/flows?namespace=${encodeURIComponent(n.namespace)}&search=${encodeURIComponent(n.name)}`)}>
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
