import React, { useState } from 'react';
import { Wrench, Play, Loader2, CheckCircle, XCircle, AlertTriangle } from 'lucide-react';
import { runTroubleshoot, TroubleshootResult } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

const STATUS_ICON: Record<string, React.ReactNode> = { pass: <CheckCircle className="w-5 h-5 text-green-400" />, fail: <XCircle className="w-5 h-5 text-red-400" />, warn: <AlertTriangle className="w-5 h-5 text-yellow-400" />, skip: <AlertTriangle className="w-5 h-5 text-slate-400" /> };
const STATUS_BG: Record<string, string> = { pass: 'border-l-green-400', fail: 'border-l-red-400', warn: 'border-l-yellow-400', skip: 'border-l-border' };

const Troubleshoot: React.FC = () => {
  usePageTitle('Troubleshoot');
  const [srcPod, setSrcPod] = useState('');
  const [dstPod, setDstPod] = useState('');
  const [ns, setNs] = useState('default');
  const [port, setPort] = useState('');
  const [results, setResults] = useState<TroubleshootResult[]>([]);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRun = async () => {
    if (!srcPod || !dstPod) return;
    setRunning(true); setError(null); setResults([]);
    try {
      const res = await runTroubleshoot({ source_pod: srcPod, dest_pod: dstPod, namespace: ns, port: port ? parseInt(port) : undefined });
      const d = res.data as Record<string, unknown>; setResults((d.results ?? d.steps ?? []) as typeof results);
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setRunning(false); }
  };

  const pass = results.filter((r) => r.status === 'pass').length;
  const fail = results.filter((r) => r.status === 'fail').length;

  return (
    <div>
      <div className="mb-6">
        <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-orange-500 to-orange-700 flex items-center justify-center shadow-lg shadow-orange-500/20"><Wrench className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Troubleshoot</h1></div>
        <p className="text-sm text-slate-400 mt-1">End-to-end connectivity troubleshooting between pods</p>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Input */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 mb-6">
        <h2 className="text-sm font-semibold text-white mb-4">Connectivity Test</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
          <input value={srcPod} onChange={(e) => setSrcPod(e.target.value)} placeholder="Source pod" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
          <input value={dstPod} onChange={(e) => setDstPod(e.target.value)} placeholder="Destination pod" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
          <input value={ns} onChange={(e) => setNs(e.target.value)} placeholder="Namespace" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
          <input value={port} onChange={(e) => setPort(e.target.value)} placeholder="Port (optional)" type="number" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
          <button onClick={handleRun} disabled={running || !srcPod || !dstPod} className="flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
            {running ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
            {running ? 'Running...' : 'Troubleshoot'}
          </button>
        </div>
      </div>

      {results.length > 0 && (
        <>
          <div className="grid grid-cols-3 gap-3 mb-4">
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Steps</div><div className="text-2xl font-bold text-white">{results.length}</div></div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Passed</div><div className="text-2xl font-bold text-green-400">{pass}</div></div>
            <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]"><div className="text-xs text-slate-400 mb-1">Failed</div><div className="text-2xl font-bold text-red-400">{fail}</div></div>
          </div>

          <div className="space-y-2">
            {results.map((r, i) => (
              <div key={i} className={`rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 border-l-4 ${STATUS_BG[r.status]} animate-fade-in`} style={{ animationDelay: `${i * 80}ms` }}>
                <div className="flex items-center gap-3">
                  {STATUS_ICON[r.status]}
                  <div className="flex-1">
                    <div className="font-medium text-white">{r.step ?? r.name ?? `Step ${i + 1}`}</div>
                    <pre className="text-sm text-slate-400 mt-1 font-mono whitespace-pre-wrap">{r.output}</pre>
                  </div>
                  <span className="text-xs text-slate-400">{r.duration_ms}ms</span>
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {results.length === 0 && !running && (
        <div className="text-center py-16">
          <Wrench className="w-16 h-16 text-slate-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-white mb-2">Ready to troubleshoot</h3>
          <p className="text-slate-400">Enter source and destination pods to run end-to-end connectivity checks.</p>
        </div>
      )}
    </div>
  );
};

export default Troubleshoot;
