import { useCallback, useEffect, useState } from 'react';
import { fetchEncryptionStatus, fetchWireGuardPeers } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function Encryption() {
  const [status, setStatus] = useState<Record<string, unknown> | null>(null);
  const [peers, setPeers] = useState<{ name?: string; status?: string; endpoint?: string }[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [e, w] = await Promise.allSettled([fetchEncryptionStatus(), fetchWireGuardPeers()]);
      if (e.status === 'fulfilled') setStatus(e.value.data as Record<string, unknown>);
      if (w.status === 'fulfilled') setPeers(((w.value.data as { peers?: typeof peers }).peers) ?? []);
      const failed = [e, w].find((r) => r.status === 'rejected') as PromiseRejectedResult | undefined;
      setErr(failed ? String(failed.reason) : '');
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
        <Eyebrow>ENCRYPTION</Eyebrow>
        <h3>Encryption on the wire</h3>
        <Metrics>
          <Metric value={String(status?.mode ?? status?.status ?? '—')} label="mode" />
          <Metric value={peers.length} label="WireGuard peers" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>WIREGUARD</Eyebrow>
        {peers.length === 0 ? <Empty>No WireGuard peers reported.</Empty> : null}
        <div className="list">
          {peers.map((p, i) => (
            <div className="agent wide" key={p.name || i}>
              <b>{p.name || `peer-${i}`}</b>
              <span>{p.status || '—'}</span>
              <small>{p.endpoint || '—'}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
