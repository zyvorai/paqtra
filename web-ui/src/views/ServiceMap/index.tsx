import { useCallback, useEffect, useState } from 'react';
import { fetchServiceMap } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Node = { id?: string; name?: string; namespace?: string };
type Edge = { source?: string; target?: string; protocol?: string };

export default function ServiceMap() {
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const { data } = await fetchServiceMap();
      setNodes(data.nodes ?? []);
      setEdges(data.edges ?? []);
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
        <Eyebrow>SERVICE MAP</Eyebrow>
        <h3>Service dependencies</h3>
        <Metrics>
          <Metric value={nodes.length} label="services" />
          <Metric value={edges.length} label="edges" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={2}>
        <Eyebrow>SERVICES</Eyebrow>
        {nodes.length === 0 ? <Empty>No services mapped yet.</Empty> : null}
        <div className="list">
          {nodes.slice(0, 50).map((n, i) => (
            <div className="agent wide" key={n.id || i}>
              <b>{n.name || n.id || '—'}</b>
              <span>{n.namespace || '—'}</span>
            </div>
          ))}
        </div>
      </Card>
      <Card span={2}>
        <Eyebrow>EDGES</Eyebrow>
        {edges.length === 0 ? <Empty>No edges yet.</Empty> : null}
        <div className="list">
          {edges.slice(0, 50).map((e, i) => (
            <div className="agent wide" key={i}>
              <b>
                {e.source || '—'} → {e.target || '—'}
              </b>
              <span>{e.protocol || '—'}</span>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
