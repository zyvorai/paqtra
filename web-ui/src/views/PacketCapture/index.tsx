import { useCallback, useEffect, useState } from 'react';
import { fetchCaptureSessions } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Session = {
  id?: string;
  node?: string;
  status?: string;
  filter?: string;
  started_at?: string;
};

export default function PacketCapture() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setSessions(((await fetchCaptureSessions()).data as { sessions?: Session[] }).sessions ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const active = sessions.filter((s) => (s.status || '').toLowerCase() === 'running' || (s.status || '').toLowerCase() === 'active').length;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}
      <Card span={3}>
        <Eyebrow>CAPTURE</Eyebrow>
        <h3>Watch the wire, live</h3>
        <Metrics>
          <Metric value={sessions.length} label="sessions" />
          <Metric value={active} label="active" />
        </Metrics>
        <p>Filtered, time-bounded captures — observe-only; never affects the datapath verdict.</p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>SESSIONS</Eyebrow>
        {sessions.length === 0 ? <Empty>No capture sessions yet.</Empty> : null}
        <div className="list">
          {sessions.map((s, i) => (
            <div className="agent wide" key={s.id || i}>
              <b>{s.node || s.id || `session-${i}`}</b>
              <span>{s.status || '—'}</span>
              <small>
                {s.filter || 'no filter'} · {s.started_at ? new Date(s.started_at).toLocaleString() : '—'}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
