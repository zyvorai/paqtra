import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { fetchClusterHealth, fetchEbpfDrops } from '../services/api';

export type Digest = {
  severity: string;
  fingerprint: string;
  changed: boolean;
  headline: string;
  whyChanged: string[];
};

export function digestTooltip(d: Digest): string {
  if (!d.changed) return d.headline || 'On-call digest';
  const why = (d.whyChanged || []).join('; ');
  return why ? `${d.headline} — ${why}` : d.headline;
}

export function digestLabel(d: Digest): string {
  const sev = (d.severity || 'info').toLowerCase();
  const fp = d.fingerprint ? d.fingerprint.slice(0, 6) : '';
  if (d.changed) return `${sev} · changed ${fp}`.trim();
  return fp ? `${sev} · ${fp}` : sev;
}

export function buildDigest(opts: {
  score?: number;
  status?: string;
  drops?: number;
  agentsOk?: boolean;
}): Digest {
  const score = opts.score ?? 100;
  const drops = opts.drops ?? 0;
  const status = (opts.status || 'healthy').toLowerCase();
  const why: string[] = [];
  let severity = 'info';
  if (status.includes('unhealthy') || score < 50) {
    severity = 'critical';
    why.push(`cluster ${status}`);
  } else if (status.includes('degraded') || score < 80) {
    severity = 'warning';
    why.push(`health score ${score}`);
  }
  if (drops > 1000) {
    severity = severity === 'info' ? 'warning' : severity;
    why.push(`${drops} drop events`);
  }
  const fingerprint = `${score}-${drops}-${status}`.slice(0, 12);
  return {
    severity,
    fingerprint,
    changed: why.length > 0,
    headline: why.length ? why[0] : `Cluster ${status} · score ${score}`,
    whyChanged: why,
  };
}

/** Client-side digest from health + drops (until a dedicated digest API exists). */
export async function loadDigest(): Promise<Digest | null> {
  try {
    const [h, d] = await Promise.allSettled([fetchClusterHealth(), fetchEbpfDrops()]);
    const health = h.status === 'fulfilled' ? h.value.data : null;
    const drops = d.status === 'fulfilled' ? d.value.data.total_drops ?? 0 : 0;
    return buildDigest({
      score: health?.score ?? health?.health_score,
      status: health?.overall ?? health?.status,
      drops,
    });
  } catch {
    return null;
  }
}

export default function DigestChip({ onOpen }: { onOpen?: () => void }) {
  const navigate = useNavigate();
  const [digest, setDigest] = useState<Digest | null>(null);

  useEffect(() => {
    let alive = true;
    const load = () => {
      loadDigest().then((d) => {
        if (alive) setDigest(d);
      });
    };
    load();
    const t = setInterval(load, 30000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, []);

  if (!digest) return null;
  const sev = (digest.severity || 'info').toLowerCase();
  return (
    <button
      type="button"
      className={`digest-chip ${sev}${digest.changed ? ' changed' : ''}`}
      onClick={() => (onOpen ? onOpen() : navigate('/'))}
      title={digestTooltip(digest)}
      aria-label={`Incident digest ${digestLabel(digest)}`}
    >
      {digestLabel(digest)}
    </button>
  );
}
