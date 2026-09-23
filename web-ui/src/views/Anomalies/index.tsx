import { useCallback, useEffect, useState } from 'react';
import { fetchAnomalies } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Anomaly = {
  id?: string;
  kind?: string;
  severity?: string;
  message?: string;
  namespace?: string;
  detected_at?: string;
};

export default function Anomalies() {
  const [items, setItems] = useState<Anomaly[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const { data } = await fetchAnomalies();
      setItems((data as { anomalies?: Anomaly[] }).anomalies ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}
      <Card span={3}>
        <Eyebrow>ANOMALIES</Eyebrow>
        <h3>Behavior that stands out</h3>
        <Metrics>
          <Metric value={items.length} label="findings" />
        </Metrics>
        <p>Review-only remediations — Paqtra does not auto-enforce.</p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>FINDINGS</Eyebrow>
        {items.length === 0 ? <Empty>No anomalies detected.</Empty> : null}
        <div className="list">
          {items.map((a, i) => (
            <div className="agent wide" key={a.id || i}>
              <b>{a.kind || a.message || `anomaly-${i}`}</b>
              <span className={`severity-badge ${(a.severity || 'info').toLowerCase()}`}>{a.severity || '—'}</span>
              <small>
                {a.namespace || '—'} · {a.detected_at ? new Date(a.detected_at).toLocaleString() : '—'}
                {a.message && a.kind ? ` · ${a.message}` : ''}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
