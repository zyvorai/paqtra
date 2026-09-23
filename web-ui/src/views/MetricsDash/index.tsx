import { useCallback, useEffect, useState } from 'react';
import { fetchMetricsSummary, fetchCiliumStatus, fetchEbpfSummary } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function MetricsDash() {
  const [metrics, setMetrics] = useState<Record<string, unknown> | null>(null);
  const [agents, setAgents] = useState(0);
  const [ebpf, setEbpf] = useState<Record<string, unknown> | null>(null);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [m, c, e] = await Promise.allSettled([
        fetchMetricsSummary(),
        fetchCiliumStatus(),
        fetchEbpfSummary(),
      ]);
      if (m.status === 'fulfilled') setMetrics(m.value.data as Record<string, unknown>);
      if (c.status === 'fulfilled') setAgents(((c.value.data as { agents?: unknown[] }).agents ?? []).length);
      if (e.status === 'fulfilled') setEbpf(e.value.data as Record<string, unknown>);
      const failed = [m, c, e].find((r) => r.status === 'rejected') as PromiseRejectedResult | undefined;
      setErr(failed ? String(failed.reason) : '');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 10000);
    return () => clearInterval(t);
  }, [load]);

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>SCORECARD</Eyebrow>
        <h3>One board for the shift</h3>
        <Metrics>
          <Metric
            value={typeof metrics?.requests_per_sec === 'number' ? Number(metrics.requests_per_sec).toFixed(1) : '—'}
            label="req/s"
          />
          <Metric
            value={typeof metrics?.avg_latency_ms === 'number' ? Number(metrics.avg_latency_ms).toFixed(1) : '—'}
            label="avg latency ms"
          />
          <Metric
            value={
              typeof metrics?.error_rate === 'number'
                ? `${(Number(metrics.error_rate) * 100).toFixed(2)}%`
                : '—'
            }
            label="error rate"
          />
          <Metric value={agents} label="Cilium agents" />
          <Metric value={Number(ebpf?.programs_total ?? ebpf?.programs ?? 0) || '—'} label="BPF programs" />
          <Metric value={Number(ebpf?.maps_total ?? ebpf?.maps ?? 0) || '—'} label="BPF maps" />
        </Metrics>
        {!metrics && !err ? <Empty>Waiting for metrics…</Empty> : null}
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
    </Board>
  );
}
