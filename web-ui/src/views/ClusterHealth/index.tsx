import { useCallback, useEffect, useState } from 'react';
import { fetchClusterHealth, type ClusterHealthSummary } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function ClusterHealth() {
  const [health, setHealth] = useState<ClusterHealthSummary | null>(null);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setHealth((await fetchClusterHealth()).data);
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

  const score = health?.score ?? health?.health_score ?? 0;
  const status = health?.overall ?? health?.status ?? '—';

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>HEALTH PULSE</Eyebrow>
        <Metrics>
          <Metric value={score} label="health score /100" />
          <Metric value={status} label="status" />
          <Metric value={health?.node_count ?? health?.kubernetes?.nodes ?? 0} label="nodes" />
          <Metric value={health?.pod_count ?? health?.kubernetes?.pods ?? 0} label="pods" />
          <Metric value={health?.endpoint_count ?? health?.kubernetes?.endpoints ?? 0} label="endpoints" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>COMPONENTS</Eyebrow>
        <h3>Subsystem status</h3>
        {!health ? <Empty>Waiting for cluster health…</Empty> : null}
        {(health?.components ?? []).length === 0 && health ? (
          <Empty>No component details yet.</Empty>
        ) : null}
        <div className="list">
          {(health?.components ?? []).map((c) => (
            <div className="agent wide" key={c.name}>
              <b>{c.name}</b>
              <span className={`severity-badge ${c.status === 'healthy' ? 'info' : c.status === 'degraded' ? 'warning' : 'critical'}`}>
                {c.status}
              </span>
              <small>{c.message || '—'}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
