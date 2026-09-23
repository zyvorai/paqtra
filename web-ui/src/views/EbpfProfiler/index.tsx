import { useCallback, useEffect, useState } from 'react';
import { fetchRealEbpfPrograms, fetchRealEbpfMaps, fetchEbpfSummary } from '../../services/api';
import { Board, Card, Eyebrow, Metric, Metrics, Warning, Empty, Toolbar } from '../../components/Board';

type Prog = { id?: string | number; name?: string; type?: string };
type MapInfo = { id?: number | string; name?: string; type?: string; entries?: number };

export default function EbpfProfiler() {
  const [programs, setPrograms] = useState<Prog[]>([]);
  const [maps, setMaps] = useState<MapInfo[]>([]);
  const [summary, setSummary] = useState<Record<string, unknown> | null>(null);
  const [err, setErr] = useState('');

  const load = useCallback(async () => {
    try {
      const [p, m, s] = await Promise.allSettled([
        fetchRealEbpfPrograms(),
        fetchRealEbpfMaps(),
        fetchEbpfSummary(),
      ]);
      if (p.status === 'fulfilled') setPrograms((p.value.data.programs as Prog[]) ?? []);
      if (m.status === 'fulfilled') setMaps((m.value.data.maps as MapInfo[]) ?? []);
      if (s.status === 'fulfilled') setSummary(s.value.data as Record<string, unknown>);
      const failed = [p, m, s].find((r) => r.status === 'rejected') as PromiseRejectedResult | undefined;
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
        <Eyebrow>eBPF</Eyebrow>
        <h3>Observe everywhere</h3>
        <Metrics>
          <Metric value={programs.length || Number(summary?.programs_total ?? 0) || '—'} label="programs" />
          <Metric value={maps.length || Number(summary?.maps_total ?? 0) || '—'} label="maps" />
        </Metrics>
        <p>Read-only Cilium program and map inventory — never mutates pins.</p>
        <Toolbar>
          <button type="button" className="btn-refresh" onClick={() => void load()}>
            Refresh
          </button>
        </Toolbar>
      </Card>
      <Card span={2}>
        <Eyebrow>PROGRAMS</Eyebrow>
        {programs.length === 0 ? <Empty>No programs listed.</Empty> : null}
        <div className="list">
          {programs.slice(0, 40).map((p, i) => (
            <div className="agent wide" key={String(p.id ?? i)}>
              <b>{p.name || `prog-${i}`}</b>
              <span>{p.type || '—'}</span>
            </div>
          ))}
        </div>
      </Card>
      <Card span={2}>
        <Eyebrow>MAPS</Eyebrow>
        {maps.length === 0 ? <Empty>No maps listed.</Empty> : null}
        <div className="list">
          {maps.slice(0, 40).map((m, i) => (
            <div className="agent wide" key={m.id ?? i}>
              <b>{m.name || `map-${i}`}</b>
              <span>{m.type || '—'}</span>
              <small>{m.entries != null ? `${m.entries} entries` : ''}</small>
            </div>
          ))}
        </div>
      </Card>
    </Board>
  );
}
