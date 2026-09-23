import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { fetchEbpfDrops } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type DropEntry = {
  reason?: string;
  reason_code?: number;
  count?: number;
  bytes?: number;
};

export default function DropDashboard() {
  const [drops, setDrops] = useState<DropEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const { data } = await fetchEbpfDrops();
      setDrops((data.drops as DropEntry[]) ?? []);
      setTotal(data.total_drops ?? 0);
      setErr('');
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 5000);
    return () => clearInterval(t);
  }, [load]);

  const top = [...drops].sort((a, b) => (b.count ?? 0) - (a.count ?? 0)).slice(0, 12);

  return (
    <Board>
      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      <Card span={3}>
        <Eyebrow>DROP PULSE</Eyebrow>
        <Metrics>
          <Metric value={total} label="total drop events" />
          <Metric value={drops.length} label="reasons seen" />
          <Metric value={top[0]?.reason ?? '—'} label="top reason" />
          <Metric value={top[0]?.count ?? 0} label="top count" />
        </Metrics>
        <p>
          Full correlation lives on <Link to="/rootcause">Root Cause</Link>. Paqtra reads Cilium drop maps —
          it never attaches a competing drop path.
        </p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>

      <Card span={3}>
        <Eyebrow>REASONS</Eyebrow>
        <h3>By Cilium drop reason</h3>
        {top.length === 0 ? <Empty>No drop entries yet.</Empty> : null}
        <div className="datatable-scroll">
          <div className="datahead" style={{ gridTemplateColumns: '2fr 1fr 1fr 0.6fr' }}>
            <span>REASON</span>
            <span>COUNT</span>
            <span>BYTES</span>
            <span>CODE</span>
          </div>
          {top.map((d, i) => (
            <div
              className="datarow"
              key={`${d.reason}-${i}`}
              style={{ gridTemplateColumns: '2fr 1fr 1fr 0.6fr' }}
            >
              <span>{d.reason ?? 'unknown'}</span>
              <span>{(d.count ?? 0).toLocaleString()}</span>
              <span>{(d.bytes ?? 0).toLocaleString()}</span>
              <span>{d.reason_code ?? '—'}</span>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
