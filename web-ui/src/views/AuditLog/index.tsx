import { useCallback, useEffect, useState } from 'react';
import { fetchAuditLog, type AuditEntry } from '../../services/api';
import { Board, Card, Eyebrow, Warning, Empty, Toolbar } from '../../components/Board';

export default function AuditLog() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [err, setErr] = useState('');
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    try {
      setEntries((await fetchAuditLog()).data.entries ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = search
    ? entries.filter(
        (e) =>
          e.action?.toLowerCase().includes(search.toLowerCase()) ||
          e.actor?.toLowerCase().includes(search.toLowerCase()) ||
          e.resource?.toLowerCase().includes(search.toLowerCase()) ||
          e.details?.toLowerCase().includes(search.toLowerCase()),
      )
    : entries;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>CONTROL PLANE</Eyebrow>
        <h3>Paqtra audit trail</h3>
        <p>Console and policy actions — observe-only evidence for the shift.</p>
        <Toolbar>
          <label>
            Search
            <input value={search} placeholder="action, actor, resource" onChange={(e) => setSearch(e.target.value)} />
          </label>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>EVENTS</Eyebrow>
        <h3>Recent actions</h3>
        {filtered.length === 0 ? <Empty>No audit events recorded yet.</Empty> : null}
        {filtered.length > 0 ? (
          <div className="datatable-scroll">
            <div className="datahead audit">
              <span>TIME</span>
              <span>ACTOR</span>
              <span>ACTION</span>
              <span>TARGET</span>
            </div>
            {filtered.slice(0, 200).map((x, i) => (
              <div className="datarow audit" key={x.id ?? i}>
                <span>{x.timestamp ? new Date(x.timestamp).toLocaleString() : '—'}</span>
                <span className="truncate" title={x.actor}>
                  {x.actor || '—'}
                </span>
                <span>{x.action || '—'}</span>
                <span className="truncate" title={x.resource || x.details || ''}>
                  {x.resource || x.details || '—'}
                </span>
              </div>
            ))}
          </div>
        ) : null}
      </Card>
    </Board>
  );
}
