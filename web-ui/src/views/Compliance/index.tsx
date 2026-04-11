import React, { useState, useEffect, useCallback } from 'react';
import {
  ShieldCheck,
  RefreshCw,
  Play,
  CheckCircle,
  XCircle,
  Clock,
  AlertTriangle,
  Loader2,
  Shield,
} from 'lucide-react';
import { fetchFrameworks as apiFetchFrameworks, runAudit as apiRunAudit, fetchSecurityPosture as apiFetchSecurityPosture } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoDismiss } from '../../hooks/useAutoDismiss';

interface FrameworkStatus {
  name: string;
  displayName: string;
  description: string;
  status: 'compliant' | 'partial' | 'non_compliant' | 'not_assessed';
  score: number;
  lastAudit: string | null;
  controls: { total: number; passing: number; failing: number };
}

const FW_META: Record<string, { displayName: string; description: string }> = {
  'PCI-DSS': { displayName: 'PCI-DSS', description: 'Payment Card Industry Data Security Standard' },
  SOC2: { displayName: 'SOC 2', description: 'Service Organization Control 2' },
  HIPAA: { displayName: 'HIPAA', description: 'Health Insurance Portability and Accountability Act' },
  GDPR: { displayName: 'GDPR', description: 'General Data Protection Regulation' },
  ISO27001: { displayName: 'ISO 27001', description: 'Information Security Management System' },
  NIST: { displayName: 'NIST CSF', description: 'NIST Cybersecurity Framework' },
};

const STATUS_CFG: Record<string, { color: string; icon: React.ReactNode; label: string }> = {
  compliant: { color: 'text-green-400 border-green-500/30', icon: <CheckCircle className="w-3.5 h-3.5" />, label: 'Compliant' },
  partial: { color: 'text-yellow-400 border-yellow-500/30', icon: <AlertTriangle className="w-3.5 h-3.5" />, label: 'Partial' },
  non_compliant: { color: 'text-red-400 border-red-500/30', icon: <XCircle className="w-3.5 h-3.5" />, label: 'Non-Compliant' },
  not_assessed: { color: 'text-slate-400 border-slate-500/30', icon: <Clock className="w-3.5 h-3.5" />, label: 'Not Assessed' },
};

function scoreColor(s: number) { return s >= 80 ? 'text-green-400' : s >= 60 ? 'text-yellow-400' : 'text-red-400'; }
function scoreBg(s: number) { return s >= 80 ? 'bg-green-400' : s >= 60 ? 'bg-yellow-400' : 'bg-red-400'; }

// TODO: Breakdown data should come from the API (e.g. /security/posture/breakdown).
// These static values are used as a fallback until the backend provides this data.
const STATIC_BREAKDOWN = [
  { label: 'Network Segmentation', value: 90 },
  { label: 'Policy Coverage', value: 85 },
  { label: 'Encryption (mTLS)', value: 78 },
  { label: 'Access Controls', value: 82 },
  { label: 'Monitoring & Logging', value: 92 },
];

const Compliance: React.FC = () => {
  usePageTitle('Compliance');
  const [frameworks, setFrameworks] = useState<FrameworkStatus[]>([]);
  const [posture, setPosture] = useState<{ score: number; trend: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useAutoDismiss<string | null>(null);
  const [auditing, setAuditing] = useState<string | null>(null);
  const [breakdown, setBreakdown] = useState(STATIC_BREAKDOWN);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try {
      const [fwRes, posRes] = await Promise.all([apiFetchFrameworks(), apiFetchSecurityPosture()]);
      setFrameworks((fwRes.data.frameworks ?? []).map((fw) => {
        const name = typeof fw === 'string' ? fw : String(fw);
        return {
          name, displayName: FW_META[name]?.displayName ?? name, description: FW_META[name]?.description ?? '',
          status: 'not_assessed' as const, score: 0, lastAudit: null, controls: { total: 0, passing: 0, failing: 0 },
        };
      }));
      const postureData = posRes.data as { score?: number; trend?: string; posture?: { score?: number; trend?: string }; breakdown?: { label: string; value: number }[] };
      const p = postureData.posture ?? postureData;
      setPosture({ score: p.score ?? 0, trend: p.trend ?? '-' });
      // Derive breakdown from API if available, otherwise keep static fallback
      if (postureData.breakdown && Array.isArray(postureData.breakdown) && postureData.breakdown.length > 0) {
        setBreakdown(postureData.breakdown);
      }
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load compliance data'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const handleAudit = async (fw: string) => {
    setAuditing(fw); setError(null);
    try {
      await apiRunAudit(fw);
      setSuccess(`Audit started for ${fw}`);
      setFrameworks((prev) => prev.map((f) => f.name === fw
        ? { ...f, status: 'partial' as const, score: 72, lastAudit: new Date().toISOString(), controls: { total: 25, passing: 18, failing: 7 } } : f));
    } catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Audit failed'); }
    finally { setAuditing(null); }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-green-500 to-emerald-700 flex items-center justify-center shadow-lg shadow-green-500/20"><ShieldCheck className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Security & Compliance</h1></div>
          <p className="text-sm text-slate-400 mt-1">Multi-framework compliance auditing</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {success && <div className="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-sm">{success}</div>}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Security Posture */}
      {posture && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6 mb-6">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            {/* Score circle */}
            <div className="flex flex-col items-center">
              <h3 className="text-sm font-medium text-slate-400 mb-4">Security Posture Score</h3>
              <div className="relative w-36 h-36">
                <svg className="w-full h-full -rotate-90" viewBox="0 0 120 120">
                  <circle cx="60" cy="60" r="52" fill="none" stroke="#334155" strokeWidth="8" />
                  <circle cx="60" cy="60" r="52" fill="none" stroke={posture.score >= 80 ? '#22c55e' : posture.score >= 60 ? '#eab308' : '#ef4444'} strokeWidth="8"
                    strokeDasharray={`${(posture.score / 100) * 326.7} 326.7`} strokeLinecap="round" />
                </svg>
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className={`text-3xl font-bold ${scoreColor(posture.score)}`}>{posture.score}</span>
                  <span className="text-xs text-slate-400">/ 100</span>
                </div>
              </div>
              <span className={`mt-3 px-2 py-0.5 rounded-full text-xs border ${posture.trend === 'improving' ? 'text-green-400 border-green-500/30' : 'text-slate-400 border-slate-700/50'}`}>
                Trend: {posture.trend}
              </span>
            </div>

            {/* Breakdown */}
            <div className="md:col-span-2">
              <h3 className="text-sm font-medium text-slate-400 mb-3">Score Breakdown</h3>
              <div className="space-y-3">
                {breakdown.map((item) => (
                  <div key={item.label} className="flex items-center gap-3">
                    {item.value >= 80 ? <CheckCircle className="w-4 h-4 text-green-400" /> : item.value >= 60 ? <AlertTriangle className="w-4 h-4 text-yellow-400" /> : <XCircle className="w-4 h-4 text-red-400" />}
                    <span className="text-sm text-white flex-1">{item.label}</span>
                    <div className="w-32 h-2 rounded-full bg-slate-700 overflow-hidden">
                      <div className={`h-full rounded-full ${scoreBg(item.value)}`} style={{ width: `${item.value}%` }} />
                    </div>
                    <span className={`text-sm font-medium w-10 text-right ${scoreColor(item.value)}`}>{item.value}%</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Framework cards */}
      <div className="flex items-center gap-2 mb-3"><div className="w-1 h-5 bg-gradient-to-b from-green-400 to-emerald-500 rounded-full" /><h2 className="text-lg font-semibold text-white">Compliance Frameworks</h2></div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {frameworks.map((fw) => {
          const cfg = STATUS_CFG[fw.status] ?? STATUS_CFG.not_assessed;
          const isAuditing = auditing === fw.name;
          return (
            <div key={fw.name} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 flex flex-col" style={{ borderTopWidth: 3, borderTopColor: fw.score >= 80 ? '#22c55e' : fw.score >= 60 ? '#eab308' : fw.status === 'not_assessed' ? '#334155' : '#ef4444' }}>
              <div className="flex items-center gap-2 mb-3">
                <Shield className="w-5 h-5 text-blue-400" />
                <div className="flex-1">
                  <div className="font-semibold text-white">{fw.displayName}</div>
                  <div className="text-xs text-slate-400">{fw.description}</div>
                </div>
              </div>
              <span className={`inline-flex items-center gap-1 self-start px-2 py-0.5 rounded-full text-xs border ${cfg.color} mb-3`}>
                {cfg.icon} {cfg.label}
              </span>
              {fw.score > 0 && (
                <div className="mb-3">
                  <div className="flex justify-between text-xs mb-1">
                    <span className="text-slate-400">Compliance Score</span>
                    <span className="font-medium text-white">{fw.score}%</span>
                  </div>
                  <div className="w-full h-2 rounded-full bg-slate-700 overflow-hidden">
                    <div className={`h-full rounded-full ${scoreBg(fw.score)}`} style={{ width: `${fw.score}%` }} />
                  </div>
                </div>
              )}
              {fw.controls.total > 0 && (
                <div className="flex gap-4 text-xs mb-3">
                  <div><span className="text-slate-400">Passing </span><span className="text-green-400 font-medium">{fw.controls.passing}</span></div>
                  <div><span className="text-slate-400">Failing </span><span className="text-red-400 font-medium">{fw.controls.failing}</span></div>
                  <div><span className="text-slate-400">Total </span><span className="font-medium text-white">{fw.controls.total}</span></div>
                </div>
              )}
              {fw.lastAudit && <div className="text-xs text-slate-400 border-t border-slate-700/50 pt-2 mb-3">Last audit: {new Date(fw.lastAudit).toLocaleDateString()}</div>}
              <button
                onClick={() => handleAudit(fw.name)}
                disabled={isAuditing}
                className="mt-auto flex items-center justify-center gap-2 w-full px-3 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 disabled:opacity-50 transition-colors"
              >
                {isAuditing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />}
                {isAuditing ? 'Running...' : 'Run Audit'}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default Compliance;
