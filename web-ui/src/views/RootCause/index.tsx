import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { fetchPacketDrops } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Drop = {
  reason?: string;
  source?: string;
  destination?: string;
  count?: number;
  suggestion?: string;
};

export default function RootCause() {
  const [drops, setDrops] = useState<Drop[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setDrops(((await fetchPacketDrops()).data as { drops?: Drop[] }).drops ?? []);
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
        <Eyebrow>ROOT CAUSE</Eyebrow>
        <h3>Why it dropped</h3>
        <Metrics>
          <Metric value={drops.length} label="findings" />
        </Metrics>
        <p>
          Correlates drops with policy evidence. Raw drop counts live on <Link to="/drops">Drops</Link>.
        </p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>FINDINGS</Eyebrow>
        {drops.length === 0 ? <Empty>No root-cause findings yet.</Empty> : null}
        <div className="list">
          {drops.slice(0, 60).map((d, i) => (
            <div className="agent wide" key={i}>
              <b>{d.reason || 'unknown'}</b>
              <span>{d.count ?? 0}</span>
              <small>
                {d.source || '—'} → {d.destination || '—'}
                {d.suggestion ? ` · ${d.suggestion}` : ''}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
