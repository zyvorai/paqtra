import { useCallback, useEffect, useState } from 'react';
import { fetchFrameworks } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Framework = {
  name?: string;
  score?: number;
  status?: string;
  controls_passed?: number;
  controls_total?: number;
};

export default function Compliance() {
  const [items, setItems] = useState<Framework[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setItems(((await fetchFrameworks()).data as { frameworks?: Framework[] }).frameworks ?? []);
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
        <Eyebrow>COMPLIANCE</Eyebrow>
        <h3>Posture, checked</h3>
        <Metrics>
          <Metric value={items.length} label="frameworks" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>FRAMEWORKS</Eyebrow>
        {items.length === 0 ? <Empty>No compliance frameworks reported.</Empty> : null}
        <div className="list">
          {items.map((f, i) => (
            <div className="agent wide" key={f.name || i}>
              <b>{f.name || `framework-${i}`}</b>
              <span>{f.score != null ? `${f.score}%` : f.status || '—'}</span>
              <small>
                {f.controls_passed != null && f.controls_total != null
                  ? `${f.controls_passed}/${f.controls_total} controls`
                  : f.status || ''}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
