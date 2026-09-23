import { useCallback, useEffect, useState } from 'react';
import { fetchIdentities, type CiliumIdentity } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function Identities() {
  const [items, setItems] = useState<CiliumIdentity[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setItems((await fetchIdentities()).data.identities ?? []);
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
        <Eyebrow>IDENTITIES</Eyebrow>
        <h3>Security identities</h3>
        <Metrics>
          <Metric value={items.length} label="identities" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>MAP</Eyebrow>
        <h3>Identity → labels</h3>
        {items.length === 0 ? <Empty>No identities reported yet.</Empty> : null}
        <div className="list">
          {items.slice(0, 100).map((id) => (
            <div className="agent wide" key={id.id}>
              <b>{id.id}</b>
              <span>{id.namespace || '—'}</span>
              <small>{(id.labels || []).slice(0, 6).join(', ') || '—'}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
