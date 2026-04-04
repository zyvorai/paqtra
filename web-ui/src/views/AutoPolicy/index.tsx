import React, { useState } from 'react';
import { Wand2, Play, Loader2, CheckCircle, AlertTriangle } from 'lucide-react';
import { generateAutopolicy } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';

interface AutoPolicyResult {
  policy_name: string;
  namespace: string;
  confidence: number;
  yaml: string;
  recommendations: string[];
  policies?: { name: string; namespace: string; spec?: Record<string, unknown> }[];
  [key: string]: unknown;
}

const AutoPolicy: React.FC = () => {
  usePageTitle('AutoPolicy');
  const [namespace, setNamespace] = useState('default');
  const [duration, setDuration] = useState('5m');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AutoPolicyResult | null>(null);

  const handleGenerate = async () => {
    setLoading(true); setError(null); setResult(null);
    try {
      const res = await generateAutopolicy({ namespace, observation_duration: duration });
      setResult(res.data);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Generation failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <div className="mb-6">
        <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-violet-500 to-purple-700 flex items-center justify-center shadow-lg shadow-violet-500/20"><Wand2 className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">AutoPolicy Engine</h1></div>
        <p className="text-sm text-slate-400 mt-1">ML-enhanced CiliumNetworkPolicy generation from observed traffic</p>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Input form */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6 mb-6">
        <h2 className="text-sm font-semibold text-white mb-4 flex items-center gap-2"><Wand2 className="w-4 h-4 text-violet-400" /> Generate Policy</h2>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div>
            <label className="block text-sm font-medium text-white mb-1">Target Namespace</label>
            <input value={namespace} onChange={(e) => setNamespace(e.target.value)} placeholder="default"
              className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
          </div>
          <div>
            <label className="block text-sm font-medium text-white mb-1">Observation Window</label>
            <select value={duration} onChange={(e) => setDuration(e.target.value)}
              className="w-full px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
              <option value="1m">1 minute</option>
              <option value="5m">5 minutes</option>
              <option value="15m">15 minutes</option>
              <option value="1h">1 hour</option>
            </select>
          </div>
          <div className="flex items-end">
            <button onClick={handleGenerate} disabled={loading}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
              {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
              {loading ? 'Analyzing...' : 'Generate Policy'}
            </button>
          </div>
        </div>
      </div>

      {/* How it works */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        {[
          { step: '1', title: 'Observe', desc: 'Monitor network flows in the target namespace using Hubble data' },
          { step: '2', title: 'Analyze', desc: 'ML engine identifies communication patterns and builds traffic graph' },
          { step: '3', title: 'Generate', desc: 'Produce least-privilege CiliumNetworkPolicy with confidence scoring' },
        ].map((s) => (
          <div key={s.step} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-8 h-8 rounded-full bg-blue-500/20 flex items-center justify-center text-blue-400 font-bold text-sm">{s.step}</div>
              <span className="font-semibold text-white">{s.title}</span>
            </div>
            <p className="text-sm text-slate-400">{s.desc}</p>
          </div>
        ))}
      </div>

      {/* Result */}
      {result && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6 animate-scale-in">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-white">Generated Policy</h2>
            <div className="flex items-center gap-2">
              {result.confidence >= 80 ? <CheckCircle className="w-4 h-4 text-green-400" /> : <AlertTriangle className="w-4 h-4 text-yellow-400" />}
              <span className={`text-sm font-medium ${result.confidence >= 80 ? 'text-green-400' : 'text-yellow-400'}`}>
                {result.confidence}% confidence
              </span>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4 mb-4">
            <div className="rounded-xl border border-slate-700/50 p-3">
              <div className="text-xs text-slate-400">Policy Name</div>
              <div className="text-sm font-medium text-white">{result.policy_name ?? result.policies?.[0]?.name ?? 'Generated Policy'}</div>
            </div>
            <div className="rounded-xl border border-slate-700/50 p-3">
              <div className="text-xs text-slate-400">Namespace</div>
              <div className="text-sm font-medium text-white">{result.namespace ?? result.policies?.[0]?.namespace ?? 'default'}</div>
            </div>
          </div>

          {(result.recommendations ?? []).length > 0 && (
            <div className="mb-4">
              <div className="text-sm font-medium text-white mb-2">Recommendations</div>
              <ul className="space-y-1">
                {(result.recommendations ?? []).map((r, i) => (
                  <li key={i} className="flex items-start gap-2 text-sm text-slate-400">
                    <span className="text-blue-400 mt-0.5">&#8226;</span> {r}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div>
            <div className="text-sm font-medium text-white mb-2">YAML Output</div>
            <pre className="p-4 rounded-xl bg-slate-950 border border-slate-800 text-xs font-mono text-slate-300 overflow-auto max-h-80">
              {result.yaml ?? JSON.stringify(result.policies?.[0]?.spec ?? {}, null, 2) ?? ''}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
};

export default AutoPolicy;
