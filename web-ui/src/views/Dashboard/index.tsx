import { useEffect, useState, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useCountUp } from '../../hooks/useCountUp';
import Reveal from '../../components/Reveal';
import DatapathHero from '../../components/DatapathHero';
import { buildDigest, type Digest } from '../../components/digest';
import { PulseFigure } from '../../components/kit/PagePulse';
import RankedList from '../../components/kit/RankedList';
import ToneDot from '../../components/kit/ToneDot';
import { countTone, scoreTone, type Tone } from '../../components/kit/tone';
import { compact } from '../../components/kit/format';
import { useRate, useSeries } from '../../components/kit/useSeries';
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

const TILE_TIMEOUT_MS = 8_000;
const STALE_AFTER_MS = 45_000;

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

function StaleBadge({ at }: { at: number | null }) {
  if (at == null) return null;
  // Staleness is measured against the current time on every render.
  // eslint-disable-next-line react-hooks/purity
  const age = Date.now() - at;
  if (age < STALE_AFTER_MS) return null;
  const secs = Math.round(age / 1000);
  return (
    <span
      className="severity-badge warning"
      style={{ fontSize: 11, marginLeft: 8 }}
      title={`Last successful refresh ${secs}s ago`}
    >
      stale {secs}s
    </span>
  );
}

type Settled<T> = { ok: true; value: T } | { ok: false; error: string };

async function settle<T>(p: Promise<{ data: T }>, timeoutMs = TILE_TIMEOUT_MS): Promise<Settled<T>> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const raced = Promise.race([
      p.then((r) => ({ data: r.data })),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`tile timeout ${timeoutMs}ms`)), timeoutMs);
      }),
    ]);
    const { data } = await raced;
    return { ok: true, value: data };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  } finally {
    if (timer) clearTimeout(timer);
  }
}

type TileKey = 'cluster' | 'digest' | 'flows' | 'ebpf' | 'platform';

export default function Overview() {
  const navigate = useNavigate();
  const [nodes, setNodes] = useState<number | null>(null);
  const [endpoints, setEndpoints] = useState<number | null>(null);
  const [endpointList, setEndpointList] = useState<{ name?: string; namespace?: string; status?: string }[]>([]);
  const [agents, setAgents] = useState<number | null>(null);
  const [clusterStatus, setClusterStatus] = useState<string>('—');
  const [clusterScore, setClusterScore] = useState<number | null>(null);
  const [ebpf, setEbpf] = useState<Record<string, unknown> | null>(null);
  const [drops, setDrops] = useState<number | null>(null);
  const [flows, setFlows] = useState<{ forwarded?: number; dropped?: number; total?: number } | null>(null);
  const [metrics, setMetrics] = useState<Record<string, unknown> | null>(null);
  const [digest, setDigest] = useState<Digest | null>(null);
  const [err, setErr] = useState('');
  const [freshAt, setFreshAt] = useState<Partial<Record<TileKey, number>>>({});
  const [, setTick] = useState(0);

  useEffect(() => {
    const t = setInterval(() => setTick((n) => n + 1), 5_000);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const failed: string[] = [];
    let dropCount = 0;
    let score = 100;
    let status = 'healthy';
    let gotHealth = false;
    let gotDrops = false;

    const bumpDigest = () => {
      if (!gotHealth && !gotDrops) return;
      setDigest(buildDigest({ score, status, drops: dropCount }));
    };
    const noteErr = (msg: string) => {
      failed.push(msg);
      if (!cancelled) setErr(failed[0] ?? '');
    };
    const mark = (key: TileKey) => {
      if (!cancelled) setFreshAt((prev) => ({ ...prev, [key]: Date.now() }));
    };

    const load = () => {
      // Paint each tile as its API returns — do not wait on Promise.all.
      // Per-tile timeout keeps one slow kubectl from blanking the page.
      void settle(fetchNodes()).then((n) => {
        if (cancelled) return;
        if (n.ok) {
          setNodes((n.value as { nodes?: unknown[] }).nodes?.length ?? 0);
          mark('cluster');
        } else noteErr(n.error);
      });
      void settle(fetchEndpoints()).then((e) => {
        if (cancelled) return;
        if (e.ok) {
          const list = (e.value as { endpoints?: { name?: string; namespace?: string; status?: string }[] }).endpoints ?? [];
          setEndpoints(list.length);
          setEndpointList(list);
          mark('cluster');
        } else noteErr(e.error);
      });
      void settle(fetchCiliumStatus()).then((c) => {
        if (cancelled) return;
        if (c.ok) {
          setAgents(((c.value as { agents?: unknown[] }).agents ?? []).length);
          mark('cluster');
        } else noteErr(c.error);
      });
      void settle(fetchEbpfSummary()).then((s) => {
        if (cancelled) return;
        if (s.ok) {
          setEbpf(s.value as Record<string, unknown>);
          mark('ebpf');
        } else noteErr(s.error);
      });
      void settle(fetchEbpfDrops()).then((d) => {
        if (cancelled) return;
        if (d.ok) {
          dropCount = (d.value as { total_drops?: number }).total_drops ?? 0;
          setDrops(dropCount);
          gotDrops = true;
          mark('digest');
          bumpDigest();
        } else noteErr(d.error);
      });
      void settle(fetchClusterHealth()).then((h) => {
        if (cancelled) return;
        if (h.ok) {
          const hv = h.value as {
            status?: string;
            overall?: string;
            score?: number;
            health_score?: number;
          };
          status = String(hv.status ?? hv.overall ?? 'ok');
          score = hv.score ?? hv.health_score ?? 100;
          setClusterStatus(status);
          setClusterScore(score);
          gotHealth = true;
          mark('cluster');
          mark('digest');
          bumpDigest();
        } else noteErr(h.error);
      });
      void settle(fetchFlowStats()).then((f) => {
        if (cancelled) return;
        if (f.ok) {
          const stats = f.value as {
            forwarded?: number;
            dropped?: number;
            total?: number;
            total_flows?: number;
            verdicts?: { forwarded?: number; dropped?: number };
          };
          setFlows({
            forwarded: stats.forwarded ?? stats.verdicts?.forwarded ?? 0,
            dropped: stats.dropped ?? stats.verdicts?.dropped ?? 0,
            total: stats.total ?? stats.total_flows,
          });
          mark('flows');
        } else noteErr(f.error);
      });
      void settle(fetchMetricsSummary()).then((m) => {
        if (cancelled) return;
        if (m.ok) {
          setMetrics(m.value as Record<string, unknown>);
          mark('platform');
        } else noteErr(m.error);
      });
    };

    load();
    const t = setInterval(load, 15_000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  // API get_ebpf_summary uses total_programs / total_maps; accept both names.
  const programs =
    Number(
      ebpf?.total_programs ?? ebpf?.programs_total ?? ebpf?.program_count ?? ebpf?.programs ?? 0,
    ) || 0;
  const maps =
    Number(ebpf?.total_maps ?? ebpf?.maps_total ?? ebpf?.map_count ?? ebpf?.maps ?? 0) || 0;

  const flowsTotal = flows ? (flows.total ?? (flows.forwarded ?? 0) + (flows.dropped ?? 0)) : undefined;
  const flowRate = useRate(flowsTotal, freshAt.flows);
  const dropRate = useRate(flows?.dropped, freshAt.flows);
  const flowSeries = useSeries(flowRate, flowRate);
  const dropSeries = useSeries(dropRate, dropRate);

  const byNamespace = new Map<string, number>();
  const byStatus = new Map<string, number>();
  for (const e of endpointList) {
    byNamespace.set(e.namespace || '—', (byNamespace.get(e.namespace || '—') ?? 0) + 1);
    byStatus.set(e.status || 'unknown', (byStatus.get(e.status || 'unknown') ?? 0) + 1);
  }
  const rank = (m: Map<string, number>) => [...m.entries()].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count);
  const notReady = endpointList.filter((e) => e.status && !/ready/i.test(e.status)).length;

  const healthTone: Tone = clusterScore == null ? 'idle' : digest && digest.severity !== 'info' ? 'warn' : scoreTone(clusterScore);
  const dropped = flows?.dropped ?? 0;
  const flowTone: Tone = !flows ? 'idle' : dropped ? 'warn' : 'ok';
  const flowTitle = !flows
    ? 'Waiting for Hubble flows.'
    : dropped
      ? `${compact(dropped)} of ${compact(flowsTotal ?? 0)} flows dropped.`
      : `${compact(flowsTotal ?? 0)} flows, none dropped.`;
  const apiTone: Tone = !metrics ? 'idle' : typeof metrics.error_rate === 'number' && metrics.error_rate > 0.01 ? 'warn' : 'ok';

  const topEndpoints = endpointList.slice(0, 3);
  const columns = [
    {
      title: 'Endpoints',
      nodes: (topEndpoints.length ? topEndpoints : [{ name: 'endpoints' }]).map((e) => ({
        label: e.name || 'endpoint',
        sub: e.namespace || 'pod',
        active: topEndpoints.length > 0,
      })),
    },
    {
      title: 'Cilium eBPF',
      nodes: [
        { label: 'bpf_lxc', sub: 'endpoint', active: programs > 0 },
        { label: 'bpf_host', sub: 'host', active: programs > 0 },
        { label: 'bpf_overlay', sub: 'tunnel', active: programs > 0 },
      ],
    },
    { title: 'Hubble', nodes: [{ label: 'hubble', sub: `${agents ?? 0} agent${agents === 1 ? '' : 's'}`, active: (flowsTotal ?? 0) > 0 || (agents ?? 0) > 0 }] },
    { title: 'Paqtra', nodes: [{ label: 'paqtra-api', sub: 'observe only', active: metrics != null || flows != null }] },
  ];

  return (
    <div className="overview">
      <header className="hero">
        <p className="eyebrow">CILIUM-NATIVE OBSERVABILITY</p>
        <h1>Trace every flow.</h1>
        <p>
          See where network traffic goes and why it is allowed or dropped — powered by Cilium eBPF. Paqtra observes; Cilium
          decides.
        </p>
        <div className="overview-hero-row">
          <StatusPill agents={agents} score={clusterScore} digest={digest} />
          <button type="button" className="primary" onClick={() => navigate('/investigate')}>
            Investigate a path
          </button>
          <button type="button" className="overview-link" onClick={() => navigate('/flows')}>
            Open Hubble flows ›
          </button>
        </div>
      </header>

      {err ? (
        <p className="warning">
          API unreachable or partial failure — showing only live responses. No demo data.
          <br />
          <span style={{ fontSize: 12, opacity: 0.85 }}>{err}</span>
        </p>
      ) : null}

      <section className="overview-stage" aria-labelledby="overview-stage-title">
        <div className="overview-stage__head">
          <h2 id="overview-stage-title">
            The datapath, live.
            <StaleBadge at={freshAt.flows ?? null} />
          </h2>
          <span className="overview-live">
            <i aria-hidden="true" /> refreshes every 15s
          </span>
        </div>
        <DatapathHero
          columns={columns}
          flowsPerSecond={flowRate}
          dropsPerSecond={dropRate}
          label="Live Cilium datapath: endpoints, Cilium eBPF programs, Hubble and the Paqtra API"
        />
        <div className="overview-pulse">
          <PulseFigure label="flows / s" value={flowRate} series={flowSeries} />
          <PulseFigure label="drops / s" value={dropRate} series={dropSeries} tone={(dropRate ?? 0) > 0 ? 'warn' : undefined} />
          <PulseFigure label="health score" value={clusterScore ?? undefined} tone={clusterScore == null ? undefined : scoreTone(clusterScore)} />
          <PulseFigure label="eBPF drop entries" value={drops ?? undefined} tone={drops == null ? undefined : countTone(drops)} />
        </div>
        <div className="metrics overview-totals">
          <Metric value={agents ?? '—'} label="Cilium agents" />
          <Metric value={nodes ?? '—'} label="nodes" />
          <Metric value={endpoints ?? '—'} label="endpoints" />
          <Metric value={programs || '—'} label="eBPF programs" />
          <Metric value={maps || '—'} label="eBPF maps" />
        </div>
      </section>

      <Chapter eyebrow="On-call digest" title={digest ? digest.headline : 'Waiting for signals…'} tone={healthTone} link="Open Health" to="/clusterhealth"
        figures={
          <>
            <Metric value={digest ? digest.severity : '—'} label="digest severity" />
            <Metric value={clusterScore ?? '—'} label="health score" />
            <Metric value={clusterStatus} label="cluster" />
            <Metric value={notReady} label="endpoints not ready" />
          </>
        }
      >
        <p>Cluster health, Cilium components and drop counts folded into one line for whoever is on call.</p>
        {digest && digest.whyChanged.length > 0 && (
          <ul className="overview-signals">
            {digest.whyChanged.slice(0, 3).map((w) => (
              <li key={w}>
                <span className={`severity-badge ${digest.severity}`}>{digest.severity}</span> {w}
              </li>
            ))}
          </ul>
        )}
      </Chapter>

      <Chapter eyebrow="Hubble flows" title={flowTitle} tone={flowTone} link="Open Flows" to="/flows" flip
        figures={
          <>
            <Metric value={flowsTotal ?? '—'} label="flows seen" />
            <Metric value={flows?.forwarded ?? '—'} label="forwarded" />
            <Metric value={flows?.dropped ?? '—'} label="dropped" />
            <Metric value={drops ?? '—'} label="eBPF drop entries" />
          </>
        }
      >
        <p>Every flow from Hubble with its verdict. Cilium owns the verdict; Paqtra shows where traffic went and why.</p>
      </Chapter>

      <Chapter eyebrow="eBPF maps" title={programs ? `${compact(programs)} programs, ${compact(maps)} maps — read-only.` : 'Read-only map inventory.'} tone={programs ? 'ok' : 'idle'} link="Open eBPF" to="/ebpf"
        figures={
          <>
            <Metric value={programs || '—'} label="programs" />
            <Metric value={maps || '—'} label="maps" />
            <Metric value={endpoints ?? '—'} label="identities/endpoints" />
          </>
        }
      >
        <p>Conntrack, policy map and IP cache viewers read Cilium's maps. Paqtra never writes them or attaches programs.</p>
      </Chapter>

      <Chapter eyebrow="Platform" title={metrics ? 'API pulse.' : 'Waiting for metrics summary…'} tone={apiTone} link="Open Scorecard" to="/metrics" flip
        figures={
          <>
            <Metric value={typeof metrics?.requests_per_sec === 'number' ? (metrics.requests_per_sec as number) : '—'} label="req/s" />
            <Metric value={typeof metrics?.avg_latency_ms === 'number' ? (metrics.avg_latency_ms as number) : '—'} label="avg latency ms" />
            <Metric
              value={typeof metrics?.error_rate === 'number' ? Number(((metrics.error_rate as number) * 100).toFixed(2)) : '—'}
              label="error %"
            />
          </>
        }
      >
        <p>Request rate, latency and error ratio of the Paqtra API itself.</p>
      </Chapter>

      <Reveal>
        <section className="overview-talking" aria-labelledby="overview-talking-title">
          <div className="overview-stage__head">
            <h2 id="overview-talking-title">Who is running.</h2>
            <Link className="overview-link" to="/endpoints">
              Open Endpoints ›
            </Link>
          </div>
          <div className="overview-talking__grid">
            <RankedList title="Endpoints by namespace" items={rank(byNamespace)} empty="No Cilium endpoints yet." limit={6} />
            <RankedList title="Endpoint status" items={rank(byStatus)} empty="No Cilium endpoints yet." limit={6} mono={false} />
            <div className="kit-ranked">
              <h3>Where to go next</h3>
              <div className="chips">
                <Link to="/topology">Topology</Link>
                <Link to="/policies">Policies</Link>
                <Link to="/drops">Drops</Link>
                <Link to="/nodes">Fleet</Link>
                <Link to="/anomalies">Anomalies</Link>
              </div>
            </div>
          </div>
        </section>
      </Reveal>

      <p className="overview-closing">
        Brothers with Cilium. Cilium owns CNI and policy; Paqtra is the Cilium-native observe and ops sibling; Netra remains the
        independent eBPF datapath. Paqtra never attaches its own datapath programs or writes Cilium pins.
      </p>
    </div>
  );
}

function StatusPill({ agents, score, digest }: { agents: number | null; score: number | null; digest: Digest | null }) {
  if (!agents) {
    return (
      <span className="overview-status tone-idle" role="status">
        <i aria-hidden="true" /> Waiting for Cilium agents
      </span>
    );
  }
  const tone = (digest && digest.severity !== 'info') || (score != null && score < 80) ? 'warn' : 'ok';
  return (
    <span className={`overview-status tone-${tone}`} role="status">
      <i aria-hidden="true" /> Observing · {agents} Cilium agent{agents === 1 ? '' : 's'}
    </span>
  );
}

function Chapter({
  eyebrow,
  title,
  tone,
  children,
  figures,
  link,
  to,
  flip,
}: {
  eyebrow: string;
  title: string;
  tone: Tone;
  children: ReactNode;
  figures: ReactNode;
  link: string;
  to: string;
  flip?: boolean;
}) {
  return (
    <Reveal>
      <section className={`overview-chapter${flip ? ' overview-chapter--flip' : ''}`}>
        <div className="overview-chapter__copy">
          <p className="apple-eyebrow">
            <ToneDot tone={tone} />
            {eyebrow}
          </p>
          <h2>{title}</h2>
          {children}
          <Link className="overview-link" to={to}>
            {link} ›
          </Link>
        </div>
        <div className="metrics overview-chapter__figures">{figures}</div>
      </section>
    </Reveal>
  );
}
