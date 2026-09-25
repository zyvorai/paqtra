import { useState } from 'react';
import { Link } from 'react-router-dom';
import api from '../../services/api';
import { Board, Card, Eyebrow, Empty, Warning, Toolbar } from '../../components/Board';
import TerminalFrame from '../../components/TerminalFrame';

type Step = {
  id: string;
  title: string;
  detail: string;
  confidence: 'observed' | 'inferred' | 'unavailable';
  evidence?: { kind: string; id: string }[];
};

type InvestigateResult = {
  id: string;
  likely_owner: string;
  next_actions: string[];
  steps: Step[];
  flow_ingest?: { status?: string; source?: string; indexed?: number };
  created_at?: string;
};

const CONF_COLOR: Record<string, string> = {
  observed: 'var(--success, #3dd68c)',
  inferred: 'var(--warning, #ffb020)',
  unavailable: 'var(--text-tertiary, #888)',
};

export default function InvestigatePath() {
  const [srcNs, setSrcNs] = useState('default');
  const [srcName, setSrcName] = useState('checkout');
  const [dstNs, setDstNs] = useState('default');
  const [dstName, setDstName] = useState('payments');
  const [port, setPort] = useState('443');
  const [protocol, setProtocol] = useState('TCP');
  const [windowMin, setWindowMin] = useState('60');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const [result, setResult] = useState<InvestigateResult | null>(null);
  const [bundleJson, setBundleJson] = useState('');

  async function run() {
    setBusy(true);
    setErr('');
    setBundleJson('');
    try {
      const { data } = await api.post<InvestigateResult>('/investigate/path', {
        source: { namespace: srcNs.trim(), name: srcName.trim() },
        destination: { namespace: dstNs.trim(), name: dstName.trim() },
        port: Number(port) || 0,
        protocol,
        time_window_minutes: Number(windowMin) || 60,
      });
      setResult(data);
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'response' in e
          ? (e as { response?: { data?: { error?: string } } }).response?.data?.error
          : undefined;
      setErr(msg || (e instanceof Error ? e.message : String(e)));
      setResult(null);
    } finally {
      setBusy(false);
    }
  }

  async function loadBundle() {
    if (!result?.id) return;
    try {
      const { data } = await api.get(`/investigate/bundles/${result.id}`);
      setBundleJson(JSON.stringify(data, null, 2));
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  async function exportBundle(format: 'json' | 'markdown') {
    if (!result?.id) return;
    try {
      const { data } = await api.get(`/investigate/bundles/${result.id}/export`, { params: { format } });
      const text =
        format === 'markdown'
          ? String((data as { content?: string }).content ?? '')
          : JSON.stringify((data as { bundle?: unknown }).bundle ?? data, null, 2);
      setBundleJson(text);
      const blob = new Blob([text], { type: format === 'markdown' ? 'text/markdown' : 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${result.id}.${format === 'markdown' ? 'md' : 'json'}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <Board>
      <Card span={3}>
        <Eyebrow>INVESTIGATE</Eyebrow>
        <h3>Why can&apos;t A reach B?</h3>
        <p className="empty-state" style={{ marginBottom: 12 }}>
          Trace DNS, Service/endpoints, identity, policy verdict, node path, and recent changes.
          Every step is labeled observed, inferred, or unavailable.
        </p>
        <Toolbar>
          <label>
            Source ns
            <input value={srcNs} onChange={(e) => setSrcNs(e.target.value)} />
          </label>
          <label>
            Source workload
            <input value={srcName} onChange={(e) => setSrcName(e.target.value)} />
          </label>
          <label>
            Dest ns
            <input value={dstNs} onChange={(e) => setDstNs(e.target.value)} />
          </label>
          <label>
            Dest workload
            <input value={dstName} onChange={(e) => setDstName(e.target.value)} />
          </label>
          <label>
            Port
            <input value={port} onChange={(e) => setPort(e.target.value)} />
          </label>
          <label>
            Proto
            <input value={protocol} onChange={(e) => setProtocol(e.target.value)} />
          </label>
          <label>
            Window (min)
            <input value={windowMin} onChange={(e) => setWindowMin(e.target.value)} />
          </label>
          <button type="button" className="primary" disabled={busy} onClick={() => void run()}>
            {busy ? 'Investigating…' : 'Investigate'}
          </button>
        </Toolbar>
      </Card>

      {err ? (
        <Card span={3}>
          <Warning>{err}</Warning>
        </Card>
      ) : null}

      {result ? (
        <>
          <Card>
            <Eyebrow>OWNER</Eyebrow>
            <h3>{result.likely_owner}</h3>
            <p className="empty-state">Investigation {result.id}</p>
            {result.likely_owner === 'policy' ? (
              <p style={{ marginTop: 12 }}>
                <Link to="/policies">Preview a Cilium CNP change →</Link>
              </p>
            ) : null}
          </Card>
          <Card span={2}>
            <Eyebrow>NEXT ACTIONS</Eyebrow>
            <ul style={{ margin: 0, paddingLeft: 18 }}>
              {result.next_actions.map((a) => (
                <li key={a} style={{ marginBottom: 6 }}>
                  {a}
                </li>
              ))}
            </ul>
            <button type="button" className="primary" style={{ marginTop: 12 }} onClick={() => void loadBundle()}>
              Load evidence bundle
            </button>
            <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
              <button type="button" onClick={() => void exportBundle('json')}>
                Export JSON
              </button>
              <button type="button" onClick={() => void exportBundle('markdown')}>
                Export Markdown
              </button>
            </div>
          </Card>
          <Card span={3}>
            <Eyebrow>STEPS</Eyebrow>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              {result.steps.map((s) => (
                <div key={s.id} style={{ borderTop: '1px solid var(--border)', paddingTop: 10 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12 }}>
                    <strong>{s.title}</strong>
                    <span style={{ color: CONF_COLOR[s.confidence] || undefined, fontSize: 12 }}>
                      {s.confidence}
                    </span>
                  </div>
                  <p className="empty-state" style={{ margin: '6px 0' }}>
                    {s.detail}
                  </p>
                  {s.evidence && s.evidence.length > 0 ? (
                    <p style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
                      evidence: {s.evidence.map((e) => `${e.kind}:${e.id}`).join(', ')}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          </Card>
          {bundleJson ? (
            <Card span={3}>
              <Eyebrow>EVIDENCE BUNDLE</Eyebrow>
              <TerminalFrame title={`investigate.bundle / ${result.id}`}>
                <pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontSize: 12 }}>{bundleJson}</pre>
              </TerminalFrame>
            </Card>
          ) : null}
        </>
      ) : !err ? (
        <Card span={3}>
          <Empty>Enter a path and run Investigate to see evidence-backed steps.</Empty>
        </Card>
      ) : null}
    </Board>
  );
}
