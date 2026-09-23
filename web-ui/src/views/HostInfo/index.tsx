import { useCallback, useEffect, useState } from 'react';
import { fetchHostInfo, type HostInfo } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function HostInfoView() {
  const [info, setInfo] = useState<HostInfo | null>(null);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      setInfo((await fetchHostInfo()).data);
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
        <Eyebrow>HOST</Eyebrow>
        <h3>Host and kernel</h3>
        {!info && !err ? <Empty>Waiting for host info…</Empty> : null}
        {info ? (
          <Metrics>
            <Metric value={info.hostname || '—'} label="hostname" />
            <Metric value={info.kernel || '—'} label="kernel" />
            <Metric value={info.os || '—'} label="os" />
            <Metric value={info.cpu_cores ?? '—'} label="CPU cores" />
            <Metric value={info.memory_total_gb ?? '—'} label="memory GB" />
            <Metric value={info.arch || '—'} label="arch" />
          </Metrics>
        ) : null}
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
    </Board>
  );
}
