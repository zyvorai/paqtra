import { useCallback, useEffect, useState } from 'react';
import { fetchPolicies, type Policy } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function Policies() {
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [err, setErr] = useState('');
  const [ns, setNs] = useState('');

  const load = useCallback(async () => {
    try {
      setPolicies((await fetchPolicies()).data.policies ?? []);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = ns ? policies.filter((p) => p.namespace === ns) : policies;
  const active = policies.filter((p) => (p.status || '').toLowerCase().includes('active') || (p.status || '').toLowerCase().includes('enforc')).length;

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}
      <Card span={3}>
        <Eyebrow>POLICIES</Eyebrow>
        <h3>Cilium workbench</h3>
        <Metrics>
          <Metric value={policies.length} label="policies" />
          <Metric value={active} label="active/enforcing" />
          <Metric value={new Set(policies.map((p) => p.namespace)).size} label="namespaces" />
        </Metrics>
        <p>Plan and apply CiliumNetworkPolicy when CRDs are present. Paqtra does not write Cilium BPF maps.</p>
        <Toolbar>
          <label>
            Namespace
            <input value={ns} placeholder="all" onChange={(e) => setNs(e.target.value)} />
          </label>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>INVENTORY</Eyebrow>
        {filtered.length === 0 ? <Empty>No policies found.</Empty> : null}
        <div className="list">
          {filtered.slice(0, 100).map((p) => (
            <div className="agent wide" key={p.id || `${p.namespace}/${p.name}`}>
              <b>
                {p.namespace}/{p.name}
              </b>
              <span>{p.status || '—'}</span>
              <small>{p.created_at ? new Date(p.created_at).toLocaleString() : '—'}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
