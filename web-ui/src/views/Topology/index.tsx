import { useCallback, useEffect, useState } from 'react';
import { fetchServiceDeps } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Dep = { source?: string; destination?: string; namespace?: string; protocol?: string; bytes?: number };

export default function Topology() {
  const [deps, setDeps] = useState<Dep[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setDeps(((await fetchServiceDeps()).data as { dependencies?: Dep[] }).dependencies ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const nodes = new Set(deps.flatMap((d) => [d.source, d.destination].filter(Boolean))).size;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}
      <Card span={3}>
        <Eyebrow>TOPOLOGY</Eyebrow>
        <h3>Observed dependency graph</h3>
        <Metrics>
          <Metric value={deps.length} label="edges" />
          <Metric value={nodes} label="nodes" />
        </Metrics>
        <p>Edges from observed traffic — not a synthetic mesh diagram.</p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>EDGES</Eyebrow>
        <h3>Source → destination</h3>
        {deps.length === 0 ? <Empty>No topology edges yet.</Empty> : null}
        <div className="list">
          {deps.slice(0, 80).map((d, i) => (
            <div className="agent wide" key={i}>
              <b>
                {d.source || '—'} → {d.destination || '—'}
              </b>
              <span>{d.protocol || '—'}</span>
              <small>
                {d.namespace || '—'} · {(d.bytes ?? 0).toLocaleString()} B
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
