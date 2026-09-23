import { useCallback, useEffect, useState } from 'react';
import { fetchCiliumStatus } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Agent = {
  name?: string;
  node?: string;
  status?: string;
  version?: string;
  message?: string;
};

export default function CiliumStatus() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setAgents(((await fetchCiliumStatus()).data as { agents?: Agent[] }).agents ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 15000);
    return () => clearInterval(t);
  }, [load]);

  const ok = agents.filter((a) => (a.status || '').toLowerCase().includes('ok') || (a.status || '').toLowerCase() === 'ready').length;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>CILIUM</Eyebrow>
        <h3>Agent status</h3>
        <Metrics>
          <Metric value={agents.length} label="agents" />
          <Metric value={ok} label="healthy" />
          <Metric value={agents.length - ok} label="other" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>PER NODE</Eyebrow>
        <h3>Datapath brotherhood</h3>
        <p>Paqtra observes these agents — it never replaces Cilium’s datapath.</p>
        {agents.length === 0 ? <Empty>No Cilium agents reported yet.</Empty> : null}
        <div className="list">
          {agents.map((a, i) => (
            <div className="agent wide" key={a.name || a.node || i}>
              <b>{a.node || a.name || `agent-${i}`}</b>
              <span>{a.status || '—'}</span>
              <small>
                {a.version || '—'} {a.message ? `· ${a.message}` : ''}
              </small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
