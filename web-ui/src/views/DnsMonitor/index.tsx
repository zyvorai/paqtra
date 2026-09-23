import { useCallback, useEffect, useState } from 'react';
import { fetchDnsQueries, fetchDnsStats } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Query = { query_name?: string; namespace?: string; rcode?: string; latency_ms?: number };
type Stats = { total?: number; failures?: number; avg_latency_ms?: number; [key: string]: unknown };

export default function DnsMonitor() {
  const [queries, setQueries] = useState<Query[]>([]);
  const [stats, setStats] = useState<Stats | null>(null);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [q, s] = await Promise.all([fetchDnsQueries(), fetchDnsStats()]);
      setQueries((q.data as { queries?: Query[] }).queries ?? []);
      setStats(s.data as Stats);
      setErr('');
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
        <Eyebrow>DNS PULSE</Eyebrow>
        <Metrics>
          <Metric value={stats?.total ?? queries.length} label="queries" />
          <Metric value={stats?.failures ?? 0} label="failures" />
          <Metric value={stats?.avg_latency_ms ?? '—'} label="avg latency ms" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>RECENT</Eyebrow>
        <h3>Observed DNS</h3>
        {queries.length === 0 ? <Empty>No DNS queries yet.</Empty> : null}
        <div className="datatable-scroll">
          <div className="datahead" style={{ gridTemplateColumns: '2fr 1fr 0.8fr 0.8fr' }}>
            <span>QUERY</span>
            <span>NAMESPACE</span>
            <span>RCODE</span>
            <span>LATENCY</span>
          </div>
          {queries.slice(0, 80).map((q, i) => (
            <div className="datarow" key={i} style={{ gridTemplateColumns: '2fr 1fr 0.8fr 0.8fr' }}>
              <span className="truncate">{q.query_name || '—'}</span>
              <span>{q.namespace || '—'}</span>
              <span>{q.rcode || '—'}</span>
              <span>{q.latency_ms != null ? `${q.latency_ms} ms` : '—'}</span>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
