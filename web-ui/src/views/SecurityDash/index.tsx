import React, { useState, useEffect, useCallback } from 'react';
import { ShieldAlert, RefreshCw, Loader2, CheckCircle, XCircle, AlertTriangle } from 'lucide-react';
import { fetchSecurityFindings, fetchZeroTrustScore, SecurityFinding, ZeroTrustScore } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import ScoreGauge from '../../components/ScoreGauge';
import ProgressBar from '../../components/ProgressBar';

const SEV_BADGE: Record<string, string> = {
  critical: 'bg-red-500/15 text-red-400 border-red-500/30',
  high: 'bg-orange-500/15 text-orange-400 border-orange-500/30',
  medium: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  low: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
};

const CAT_BADGE: Record<string, string> = {
  network_policy: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  encryption: 'bg-purple-500/15 text-purple-400 border-purple-500/30',
  identity: 'bg-cyan-500/15 text-cyan-400 border-cyan-500/30',
  least_privilege: 'bg-orange-500/15 text-orange-400 border-orange-500/30',
};

function scoreColor(s: number) { return s >= 80 ? 'text-green-400' : s >= 60 ? 'text-yellow-400' : 'text-red-400'; }

const SecurityDash: React.FC = () => {
  usePageTitle('Security Dashboard');
  const [findings, setFindings] = useState<SecurityFinding[]>([]);
  const [zt, setZt] = useState<ZeroTrustScore | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try {
      const [fRes, zRes] = await Promise.all([fetchSecurityFindings(), fetchZeroTrustScore()]);
      setFindings(fRes.data.findings ?? []);
      setZt(zRes.data);
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const criticalCount = findings.filter((f) => f.severity === 'critical').length;
  const openCount = findings.filter((f) => f.status === 'open').length;

  const pillars = zt ? [
    { label: 'Network Segmentation', value: zt.network_segmentation },
    { label: 'Identity Verification', value: zt.identity_verification },
    { label: 'Encryption', value: zt.encryption },
    { label: 'Least Privilege', value: zt.least_privilege },
    { label: 'Monitoring', value: zt.monitoring },
  ] : [];

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20"><ShieldAlert className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Security Dashboard</h1></div>
          <p className="text-sm text-slate-400 mt-1">Zero-trust security posture and vulnerability findings</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Zero Trust Score */}
      {zt && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6 mb-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div className="flex flex-col items-center">
              <h3 className="text-sm font-medium text-slate-400 mb-4">Zero Trust Score</h3>
              <ScoreGauge score={zt.overall} size={144} label="/ 100" />
              <div className="flex gap-3 mt-3">
                <span className="text-sm text-slate-400">{criticalCount} critical</span>
                <span className="text-sm text-slate-400">{openCount} open</span>
              </div>
            </div>
            <div>
              <h3 className="text-sm font-medium text-slate-400 mb-3">Zero Trust Pillars</h3>
              <div className="space-y-3">
                {pillars.map((p) => (
                  <div key={p.label}>
                    <div className="flex items-center justify-between text-sm mb-1">
                      <div className="flex items-center gap-2">
                        {p.value >= 80 ? <CheckCircle className="w-4 h-4 text-green-400" /> : p.value >= 60 ? <AlertTriangle className="w-4 h-4 text-yellow-400" /> : <XCircle className="w-4 h-4 text-red-400" />}
                        <span className="text-white">{p.label}</span>
                      </div>
                      <span className={`font-medium ${scoreColor(p.value)}`}>{p.value}%</span>
                    </div>
                    <ProgressBar value={p.value} size="lg" />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Findings */}
      <div className="flex items-center gap-2 mb-3">
        <div className="w-1 h-5 bg-gradient-to-b from-red-400 to-orange-500 rounded-full" />
        <h2 className="text-lg font-semibold text-white">Security Findings</h2>
      </div>
      <div className="space-y-3">
        {findings.map((f) => (
          <div key={f.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-start gap-4">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-2">
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${SEV_BADGE[f.severity] ?? ''}`}>{f.severity.toUpperCase()}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${CAT_BADGE[f.category] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>{f.category.replace(/_/g, ' ')}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${f.status === 'open' ? 'bg-red-500/15 text-red-400 border-red-500/30' : 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30'}`}>{f.status}</span>
                </div>
                <h3 className="font-medium text-white mb-1">{f.title}</h3>
                <p className="text-sm text-slate-400 mb-2">{f.description}</p>
                <div className="text-xs text-slate-400">
                  <span className="font-mono">{f.resource}</span> in <span className="px-1.5 py-0.5 rounded bg-slate-900/50">{f.namespace}</span>
                </div>
              </div>
              <div className="p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-sm max-w-xs flex-shrink-0">
                <div className="text-xs text-green-400 font-medium mb-1">Remediation</div>
                <div className="text-green-300 text-xs">{f.remediation}</div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default SecurityDash;
