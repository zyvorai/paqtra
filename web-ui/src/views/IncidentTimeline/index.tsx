import { useCallback, useEffect, useState } from 'react';
import { fetchIncidents } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Incident = {
  id?: string;
  title?: string;
  severity?: string;
  status?: string;
  started_at?: string;
  summary?: string;
  affected_services?: string[];
  duration?: string;
  root_cause?: string;
};

export default function IncidentTimeline() {
  const [items, setItems] = useState<Incident[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setItems(((await fetchIncidents()).data as { incidents?: Incident[] }).incidents ?? []);
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
        <Eyebrow>INCIDENTS</Eyebrow>
        <h3>When signals agree</h3>
        <Metrics>
          <Metric value={items.length} label="incidents" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>TIMELINE</Eyebrow>
        {items.length === 0 ? <Empty>No incidents joined yet.</Empty> : null}
        <div className="list">
          {items.map((x, i) => (
            <div className="agent wide" key={x.id || i}>
              <b>{x.title || x.id || `incident-${i}`}</b>
              <span className={`severity-badge ${(x.severity || 'info').toLowerCase()}`}>{x.severity || x.status || '—'}</span>
              <small>
                {x.started_at ? new Date(x.started_at).toLocaleString() : '—'}
                {x.status ? ` · ${x.status}${x.duration ? ` after ${x.duration}` : ''}` : ''}
                {x.summary ? ` · ${x.summary}` : ''}
                {x.affected_services?.length ? ` · affects ${x.affected_services.join(', ')}` : ''}
                {x.root_cause ? ` · root cause: ${x.root_cause}` : ''}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
