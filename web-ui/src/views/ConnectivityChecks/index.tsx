import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import {
  createConnectivityPath,
  deleteConnectivityPath,
  fetchConnectivityAlerts,
  fetchConnectivityPaths,
} from '../../services/api';
import { Board, Card, Eyebrow, Empty, Warning, Toolbar, Metric, Metrics } from '../../components/Board';

type PathRow = {
  path: {
    id: string;
    name: string;
    src_namespace: string;
    src_workload: string;
    dst_namespace: string;
    dst_service: string;
    port: number;
    protocol?: string;
  };
  status: {
    status?: string;
    confidence?: string;
    notes?: string[];
    evidence_flow_ids?: string[];
    investigate?: { source?: { namespace?: string; name?: string }; destination?: { namespace?: string; name?: string }; port?: number };
  };
};

export default function ConnectivityChecks() {
  const [rows, setRows] = useState<PathRow[]>([]);
  const [alerts, setAlerts] = useState<Record<string, unknown>[]>([]);
  const [err, setErr] = useState('');
  const [name, setName] = useState('checkout→payments');
  const [srcNs, setSrcNs] = useState('default');
  const [srcWl, setSrcWl] = useState('checkout');
  const [dstNs, setDstNs] = useState('default');
  const [dstSvc, setDstSvc] = useState('payments');
  const [port, setPort] = useState('443');

  const load = useCallback(async () => {
    try {
      const [p, a] = await Promise.all([fetchConnectivityPaths(), fetchConnectivityAlerts()]);
      setRows(((p.data as { paths?: PathRow[] }).paths ?? []) as PathRow[]);
      setAlerts(((a.data as { alerts?: Record<string, unknown>[] }).alerts ?? []) as Record<string, unknown>[]);
      setErr('');
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 30_000);
    return () => clearInterval(t);
  }, [load]);

  async function add() {
    try {
      await createConnectivityPath({
        name,
        src_namespace: srcNs.trim(),
        src_workload: srcWl.trim(),
        dst_namespace: dstNs.trim(),
        dst_service: dstSvc.trim(),
        port: Number(port) || 0,
        protocol: 'TCP',
      });
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  async function remove(id: string) {
    try {
      await deleteConnectivityPath(id);
      await load();
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  const regressions = rows.filter((r) => r.status?.status === 'regression' || r.status?.status === 'degraded').length;
  const unknown = rows.filter((r) => r.status?.status === 'unknown').length;

  return (
    <Board>
      <Card span={3}>
        <Eyebrow>CONNECTIVITY</Eyebrow>
        <h3>Declared service paths</h3>
        <p>Observe-only checks. Quiet traffic is unknown — never assumed healthy. No policy apply or BPF changes.</p>
        <Metrics>
          <Metric value={rows.length} label="paths" />
          <Metric value={regressions} label="regressions" />
          <Metric value={unknown} label="unknown" />
          <Metric value={alerts.length} label="alerts" />
        </Metrics>
      </Card>

      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>DECLARE</Eyebrow>
        <Toolbar>
          <label>
            Name
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            Src ns
            <input value={srcNs} onChange={(e) => setSrcNs(e.target.value)} />
          </label>
          <label>
            Src workload
            <input value={srcWl} onChange={(e) => setSrcWl(e.target.value)} />
          </label>
          <label>
            Dst ns
            <input value={dstNs} onChange={(e) => setDstNs(e.target.value)} />
          </label>
          <label>
            Dst service
            <input value={dstSvc} onChange={(e) => setDstSvc(e.target.value)} />
          </label>
          <label>
            Port
            <input value={port} onChange={(e) => setPort(e.target.value)} />
          </label>
          <button type="button" className="primary" onClick={() => void add()}>
            Add path
          </button>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>PATHS</Eyebrow>
        {rows.length === 0 ? <Empty>No declared paths yet.</Empty> : null}
        <div className="list">
          {rows.map((r) => (
            <div className="agent wide" key={r.path.id}>
              <b>{r.path.name || r.path.id}</b>
              <span>
                {r.path.src_namespace}/{r.path.src_workload} → {r.path.dst_namespace}/{r.path.dst_service}:
                {r.path.port}
              </span>
              <small>
                {r.status?.status ?? '—'} · {r.status?.confidence ?? '—'}
              </small>
              <div style={{ display: 'flex', gap: 8, marginTop: 6 }}>
                <Link
                  to="/investigate"
                  state={r.status?.investigate}
                >
                  Investigate
                </Link>
                <button type="button" onClick={() => void remove(r.path.id)}>
                  Remove
                </button>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Card span={3}>
        <Eyebrow>ALERTS</Eyebrow>
        {alerts.length === 0 ? <Empty>No sustained regressions.</Empty> : null}
        <div className="list">
          {alerts.map((a, i) => (
            <div className="agent wide" key={String(a.id ?? i)}>
              <b>{String(a.path_id ?? 'alert')}</b>
              <span>{String(a.status ?? '')}</span>
              <small>{JSON.stringify(a.summary ?? a.notes ?? '')}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
