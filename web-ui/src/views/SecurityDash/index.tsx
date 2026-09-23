import { useCallback, useEffect, useState } from 'react';
import { fetchZeroTrustScore, fetchSecurityFindings } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

export default function SecurityDash() {
  const [score, setScore] = useState<Record<string, number> | null>(null);
  const [findings, setFindings] = useState<{ title?: string; severity?: string; message?: string }[]>([]);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [z, f] = await Promise.allSettled([fetchZeroTrustScore(), fetchSecurityFindings()]);
      if (z.status === 'fulfilled') setScore(z.value.data as Record<string, number>);
      if (f.status === 'fulfilled') setFindings(((f.value.data as { findings?: typeof findings }).findings) ?? []);
      const failed = [z, f].find((r) => r.status === 'rejected') as PromiseRejectedResult | undefined;
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
        <Eyebrow>SECURITY POSTURE</Eyebrow>
        <h3>Zero-trust board</h3>
        <Metrics>
          <Metric value={score?.overall ?? '—'} label="overall" />
          <Metric value={score?.network_segmentation ?? '—'} label="segmentation" />
          <Metric value={score?.identity_verification ?? '—'} label="identity" />
          <Metric value={score?.encryption ?? '—'} label="encryption" />
          <Metric value={score?.least_privilege ?? '—'} label="least privilege" />
          <Metric value={score?.monitoring ?? '—'} label="monitoring" />
        </Metrics>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={3}>
        <Eyebrow>FINDINGS</Eyebrow>
        {findings.length === 0 ? <Empty>No security findings.</Empty> : null}
        <div className="list">
          {findings.slice(0, 40).map((f, i) => (
            <div className="agent wide" key={i}>
              <b>{f.title || f.message || `finding-${i}`}</b>
              <span className={`severity-badge ${(f.severity || 'info').toLowerCase()}`}>{f.severity || '—'}</span>
              <small>{f.message && f.title ? f.message : ''}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
