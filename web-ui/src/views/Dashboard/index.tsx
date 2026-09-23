import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useCountUp } from '../../hooks/useCountUp';
import Reveal from '../../components/Reveal';
import { buildDigest, digestLabel, type Digest } from '../../components/DigestChip';
import {
  fetchNodes,
  fetchEndpoints,
  fetchCiliumStatus,
  fetchEbpfSummary,
  fetchEbpfDrops,
  fetchClusterHealth,
  fetchFlowStats,
  fetchMetricsSummary,
} from '../../services/api';

function Metric({ value, label }: { value: number | string; label: string }) {
  const numeric = typeof value === 'number' && Number.isFinite(value);
  const animated = useCountUp(numeric ? (value as number) : 0);
  return (
    <div>
      <b>{numeric ? Math.round(animated).toLocaleString() : value}</b>
      <span>{label}</span>
    </div>
  );
}

type Settled<T> = { ok: true; value: T } | { ok: false; error: string };

async function settle<T>(p: Promise<{ data: T }>): Promise<Settled<T>> {
  try {
    const { data } = await p;
    return { ok: true, value: data };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export default function Overview() {
  const [nodes, setNodes] = useState<number | null>(null);
  const [endpoints, setEndpoints] = useState<number | null>(null);
  const [agents, setAgents] = useState<number | null>(null);
  const [clusterStatus, setClusterStatus] = useState<string>('—');
  const [clusterScore, setClusterScore] = useState<number | null>(null);
  const [ebpf, setEbpf] = useState<Record<string, unknown> | null>(null);
  const [drops, setDrops] = useState<number | null>(null);
  const [flows, setFlows] = useState<{ forwarded?: number; dropped?: number; total?: number } | null>(null);
  const [metrics, setMetrics] = useState<Record<string, unknown> | null>(null);
  const [digest, setDigest] = useState<Digest | null>(null);
  const [err, setErr] = useState('');

  useEffect(() => {
    Promise.all([
      settle(fetchNodes()),
      settle(fetchEndpoints()),
      settle(fetchCiliumStatus()),
      settle(fetchEbpfSummary()),
      settle(fetchEbpfDrops()),
      settle(fetchClusterHealth()),
      settle(fetchFlowStats()),
      settle(fetchMetricsSummary()),
    ]).then(([n, e, c, s, d, h, f, m]) => {
      const failed: string[] = [];
      if (n.ok) setNodes((n.value as { nodes?: unknown[] }).nodes?.length ?? 0);
      else failed.push(n.error);
      if (e.ok) setEndpoints((e.value as { endpoints?: unknown[] }).endpoints?.length ?? 0);
      else failed.push(e.error);
      if (c.ok) {
        const agentsList = (c.value as { agents?: unknown[] }).agents ?? [];
        setAgents(agentsList.length);
      } else failed.push(c.error);
      if (s.ok) setEbpf(s.value as Record<string, unknown>);
      else failed.push(s.error);
      let dropCount = 0;
      if (d.ok) {
        dropCount = (d.value as { total_drops?: number }).total_drops ?? 0;
        setDrops(dropCount);
      } else failed.push(d.error);
      let score = 100;
      let status = 'healthy';
      if (h.ok) {
        const hv = h.value as { status?: string; overall?: string; score?: number; health_score?: number };
        status = String(hv.status ?? hv.overall ?? 'ok');
        score = hv.score ?? hv.health_score ?? 100;
        setClusterStatus(status);
        setClusterScore(score);
      } else failed.push(h.error);
      if (f.ok) {
        const stats = f.value as {
          forwarded?: number;
          dropped?: number;
          total?: number;
          verdicts?: { forwarded?: number; dropped?: number };
        };
        setFlows({
          forwarded: stats.forwarded ?? stats.verdicts?.forwarded ?? 0,
          dropped: stats.dropped ?? stats.verdicts?.dropped ?? 0,
          total: stats.total,
        });
      } else failed.push(f.error);
      if (m.ok) setMetrics(m.value as Record<string, unknown>);
      else failed.push(m.error);
      setDigest(buildDigest({ score, status, drops: dropCount }));
      setErr(failed.length ? failed[0] : '');
    });
  }, []);

  const programs = Number(ebpf?.programs_total ?? ebpf?.program_count ?? ebpf?.programs ?? 0) || 0;
  const maps = Number(ebpf?.maps_total ?? ebpf?.map_count ?? ebpf?.maps ?? 0) || 0;

  return (
    <div className="grid">
      <Reveal>
        <section className="card span2">
          <p className="eyebrow">CILIUM DATAPATH</p>
          <h3>Brothers with Cilium.</h3>
          <p>
            Paqtra observes Hubble flows and Cilium eBPF maps — it never attaches its own datapath programs or writes
            Cilium pins. Trace every flow; Cilium owns the verdict.
          </p>
          <div className="metrics">
            <Metric value={agents ?? '—'} label="Cilium agents" />
            <Metric value={nodes ?? '—'} label="nodes" />
            <Metric value={endpoints ?? '—'} label="endpoints" />
            <Metric value={clusterStatus} label="cluster" />
          </div>
          {err ? (
            <p className="warning">
              API unreachable or partial failure — showing only live responses. No demo data.
              <br />
              <span style={{ fontSize: 12, opacity: 0.85 }}>{err}</span>
            </p>
          ) : null}
        </section>
      </Reveal>

      <Reveal delay={80}>
        <section className="card span2">
          <p className="eyebrow">ON-CALL DIGEST</p>
          <h3>{digest ? digest.headline : 'Waiting for signals…'}</h3>
          {digest ? (
            <>
              <div className="metrics">
                <Metric value={digestLabel(digest)} label="digest" />
                <Metric value={clusterScore ?? '—'} label="health score" />
                <Metric value={drops ?? '—'} label="drop events" />
              </div>
              {(digest.whyChanged || []).length > 0 ? (
                <div className="list">
                  {digest.whyChanged.map((w) => (
                    <p key={w}>
                      <span className={`severity-badge ${digest.severity}`}>{digest.severity}</span> {w}
                    </p>
                  ))}
                </div>
              ) : (
                <p className="empty-state">No digest changes — cluster looks quiet.</p>
              )}
            </>
          ) : (
            <p className="empty-state">Digest builds from cluster health and drop counts.</p>
          )}
        </section>
      </Reveal>

      <Reveal delay={120}>
        <section className="card span2">
          <p className="eyebrow">HUBBLE / FLOWS</p>
          <h3>Allow and drop at a glance</h3>
          <div className="metrics">
            <Metric value={flows?.total ?? flows?.forwarded ?? '—'} label="flows seen" />
            <Metric value={flows?.forwarded ?? '—'} label="forwarded" />
            <Metric value={flows?.dropped ?? '—'} label="dropped" />
            <Metric value={drops ?? '—'} label="eBPF drop entries" />
          </div>
          <p>
            Full tables live on <Link to="/flows">Flows</Link> and <Link to="/drops">Drops</Link>.
          </p>
        </section>
      </Reveal>

      <section className="card span3">
        <p className="eyebrow">eBPF MAPS</p>
        <h3>Read-only inventory</h3>
        <div className="metrics">
          <Metric value={programs || '—'} label="programs" />
          <Metric value={maps || '—'} label="maps" />
          <Metric value={endpoints ?? '—'} label="identities/endpoints" />
        </div>
        <p>
          Conntrack, policy map, and IP cache viewers are under Diagnostics — never mutate Cilium maps from this console.
        </p>
      </section>

      <section className="card span3">
        <p className="eyebrow">PLATFORM</p>
        <h3>API pulse</h3>
        <div className="metrics">
          <Metric
            value={typeof metrics?.requests_per_sec === 'number' ? (metrics.requests_per_sec as number) : '—'}
            label="req/s"
          />
          <Metric
            value={typeof metrics?.avg_latency_ms === 'number' ? (metrics.avg_latency_ms as number) : '—'}
            label="avg latency ms"
          />
          <Metric
            value={typeof metrics?.error_rate === 'number' ? Number(((metrics.error_rate as number) * 100).toFixed(2)) : '—'}
            label="error %"
          />
        </div>
        {!metrics && !err ? <p className="empty-state">Waiting for metrics summary…</p> : null}
        {!metrics && err ? <p className="empty-state">Metrics unavailable until the API is reachable.</p> : null}
      </section>

      <section className="card span2">
        <p className="eyebrow">INVESTIGATE</p>
        <h3>Where to go next</h3>
        <div className="chips">
          <Link to="/flows">Flows</Link>
          <Link to="/topology">Topology</Link>
          <Link to="/policies">Policies</Link>
          <Link to="/drops">Drops</Link>
          <Link to="/clusterhealth">Health</Link>
          <Link to="/nodes">Fleet</Link>
          <Link to="/ebpf">eBPF</Link>
          <Link to="/anomalies">Anomalies</Link>
        </div>
      </section>

      <section className="card span2">
        <p className="eyebrow">BROTHERHOOD</p>
        <h3>Cilium · Paqtra · Netra</h3>
        <p>
          Cilium owns CNI and policy. Paqtra is the Cilium-native observe/ops sibling. Netra remains the independent
          eBPF datapath — Paqtra does not compete with either.
        </p>
      </section>
    </div>
  );
}
