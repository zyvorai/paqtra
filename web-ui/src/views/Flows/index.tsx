import { useCallback, useEffect, useState } from 'react';
import { fetchFlows, fetchFlowStats, type Flow, type FlowEndpoint } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';
import TerminalFrame from '../../components/TerminalFrame';

function endpointLabel(ep: FlowEndpoint | string | undefined): string {
  if (!ep) return '—';
  if (typeof ep === 'string') return ep;
  const parts = [ep.namespace, ep.pod || ep.ip].filter(Boolean);
  return parts.join('/') || ep.ip || '—';
}

function flowLine(f: Flow): string {
  const src = endpointLabel(f.source);
  const dst = endpointLabel(f.destination);
  const port = f.port ? `:${f.port}` : '';
  const t = f.timestamp ? new Date(f.timestamp).toLocaleTimeString() : '';
  return `${t}  ${f.verdict || '—'}  ${f.protocol || '—'}  ${src} → ${dst}${port}`;
}

export default function Flows() {
  const [flows, setFlows] = useState<Flow[]>([]);
  const [stats, setStats] = useState<{
    total?: number;
    forwarded?: number;
    dropped?: number;
    total_flows?: number;
  } | null>(null);
  const [verdict, setVerdict] = useState('');
  const [namespace, setNamespace] = useState('');
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [f, s] = await Promise.all([
        fetchFlows({
          limit: 100,
          ...(verdict ? { verdict } : {}),
          ...(namespace ? { namespace } : {}),
        }),
        fetchFlowStats(),
      ]);
      setFlows(f.data.flows ?? []);
      setStats(s.data as typeof stats);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, [verdict, namespace]);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 5000);
    return () => clearInterval(t);
  }, [load]);

  const forwarded = stats?.forwarded ?? 0;
  const dropped = stats?.dropped ?? 0;
  const total = stats?.total ?? stats?.total_flows ?? flows.length;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={2}>
        <Eyebrow>LIVE STREAM</Eyebrow>
        <h3>Filters</h3>
        <p>Optional Cilium/Hubble enrichment. When Hubble is unavailable, use Drops and eBPF for node-local evidence.</p>
        <Toolbar>
          <label>
            Verdict
            <input value={verdict} placeholder="FORWARDED" onChange={(e) => setVerdict(e.target.value)} />
          </label>
          <label>
            Namespace
            <input value={namespace} placeholder="default" onChange={(e) => setNamespace(e.target.value)} />
          </label>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card>
        <Eyebrow>FLOW SUMMARY</Eyebrow>
        <h3>Window aggregate</h3>
        <Metrics>
          <Metric value={total} label="flows sampled" />
          <Metric value={forwarded} label="forwarded" />
          <Metric value={dropped} label="dropped" />
          <Metric value={flows.length} label="rows shown" />
        </Metrics>
      </Card>

      <div className="span3">
        <TerminalFrame title="hubble.GetFlows / filtered">
          {flows.length === 0 ? (
            <Empty>No flows yet — waiting on Hubble Relay.</Empty>
          ) : (
            <pre style={{ margin: 0, fontSize: 12, lineHeight: 1.45 }}>
              {flows.slice(0, 120).map(flowLine).join('\n')}
            </pre>
          )}
        </TerminalFrame>
      </div>
    </Board>
  );
}
