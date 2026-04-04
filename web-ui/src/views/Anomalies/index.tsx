import React, { useState, useEffect, useCallback } from 'react';
import {
  Bug,
  RefreshCw,
  Wrench,
  AlertTriangle,
  AlertOctagon,
  Info,
  CheckCircle,
  Loader2,
  X,
} from 'lucide-react';
import { fetchAnomalies as apiFetchAnomalies, remediateAnomaly as apiRemediate, Anomaly } from '../../services/api';
import { isAxiosError } from 'axios';
import { format, parseISO } from 'date-fns';
import { usePageTitle } from '../../hooks/usePageTitle';

const SEV_BADGE: Record<string, string> = {
  critical: 'bg-red-500/15 text-red-400 border-red-500/30',
  high: 'bg-orange-500/15 text-orange-400 border-orange-500/30',
  medium: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  low: 'bg-slate-500/15 text-slate-400 border-slate-500/30',
  info: 'bg-green-500/15 text-green-400 border-green-500/30',
};

const SEV_ICON: Record<string, React.ReactNode> = {
  critical: <AlertOctagon className="w-3.5 h-3.5" />,
  high: <AlertTriangle className="w-3.5 h-3.5" />,
  medium: <Info className="w-3.5 h-3.5" />,
  low: <Info className="w-3.5 h-3.5" />,
  info: <Info className="w-3.5 h-3.5" />,
};

const STATUS_BADGE: Record<string, string> = {
  open: 'bg-red-500/15 text-red-400 border-red-500/30',
  investigating: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  remediated: 'bg-green-500/15 text-green-400 border-green-500/30',
  resolved: 'bg-green-500/15 text-green-400 border-green-500/30',
  dismissed: 'bg-slate-500/15 text-slate-400 border-slate-500/30',
};

const Anomalies: React.FC = () => {
  usePageTitle('Anomalies');
  const [anomalies, setAnomalies] = useState<Anomaly[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [remId, setRemId] = useState<string | null>(null);
  const [remediating, setRemediating] = useState(false);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setAnomalies((await apiFetchAnomalies()).data.anomalies ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to fetch anomalies'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const handleRemediate = async () => {
    if (!remId) return;
    setRemediating(true); setError(null);
    try { await apiRemediate(remId); setRemId(null); setSuccess('Remediation initiated'); fetchData(); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Remediation failed'); }
    finally { setRemediating(false); }
  };

  const countSev = (s: string) => anomalies.filter((a) => a.severity === s).length;
  const totalOpen = anomalies.filter((a) => !['remediated', 'resolved', 'dismissed'].includes(a.status)).length;
  const fmtTs = (ts: string) => { try { return format(parseISO(ts), 'yyyy-MM-dd HH:mm'); } catch { return ts; } };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20"><Bug className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Anomaly Detection</h1></div>
          <p className="text-sm text-slate-400 mt-1">ML-powered network anomaly detection</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {success && <div className="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-sm">{success}</div>}

      {/* Summary */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
        <SummaryCard label="Total Anomalies" value={anomalies.length} sub={`${totalOpen} open`} gradient="stat-card-blue card-glow transition-all hover:scale-[1.02]" />
        <SummaryCard label="Critical" value={countSev('critical')} gradient="stat-card-red card-glow transition-all hover:scale-[1.02]" valueColor="text-red-400" />
        <SummaryCard label="High" value={countSev('high')} gradient="stat-card-orange card-glow transition-all hover:scale-[1.02]" valueColor="text-orange-400" />
        <SummaryCard label="Medium" value={countSev('medium')} gradient="stat-card-cyan card-glow-cyan transition-all hover:scale-[1.02]" valueColor="text-blue-400" />
      </div>

      {/* Table */}
      <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-700/50 flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
            <Bug className="w-4 h-4 text-white" />
          </div>
          <h2 className="text-lg font-semibold text-white">Detected Anomalies</h2>
        </div>
        {loading && <div className="flex justify-center p-3"><Loader2 className="w-5 h-5 animate-spin text-blue-400" /></div>}
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/50 bg-slate-900/50">
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Severity</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Description</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Type</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Source</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Detected</th>
                <th className="text-left px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Status</th>
                <th className="text-right px-4 py-3 font-semibold text-slate-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody>
              {anomalies.length === 0 && !loading ? (
                <tr>
                  <td colSpan={7} className="px-4 py-12 text-center">
                    <CheckCircle className="w-12 h-12 text-green-400 mx-auto mb-3" />
                    <div className="text-white font-medium">No anomalies detected</div>
                    <div className="text-sm text-slate-400">The ML engine is monitoring your traffic.</div>
                  </td>
                </tr>
              ) : anomalies.map((a) => (
                <tr key={a.id} className="border-b border-slate-700/30 table-row-hover">
                  <td className="px-4 py-2.5">
                    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border ${SEV_BADGE[a.severity] ?? ''}`}>
                      {SEV_ICON[a.severity]} {a.severity.toUpperCase()}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 max-w-xs">
                    <div className="font-medium text-white truncate">{a.description}</div>
                    {a.remediation && <div className="text-xs text-green-400 truncate">{a.remediation}</div>}
                  </td>
                  <td className="px-4 py-2.5"><span className="px-2 py-0.5 rounded border border-slate-700/50 text-xs">{a.anomaly_type.replace(/_/g, ' ')}</span></td>
                  <td className="px-4 py-2.5 text-slate-400">{a.source_namespace}/{a.source_pod ?? '\u2014'}</td>
                  <td className="px-4 py-2.5 text-slate-400">{fmtTs(a.detected_at)}</td>
                  <td className="px-4 py-2.5">
                    <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[a.status] ?? ''}`}>{a.status}</span>
                  </td>
                  <td className="px-4 py-2.5 text-right">
                    <button
                      disabled={a.status === 'remediated' || a.status === 'resolved'}
                      onClick={() => setRemId(a.id)}
                      className="inline-flex items-center gap-1 px-3 py-1.5 rounded-lg border border-slate-700/50 text-xs hover:bg-slate-700/30 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                    >
                      <Wrench className="w-3 h-3" /> Remediate
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Remediate confirm */}
      {remId && (
        <div className="fixed inset-0 z-50 modal-backdrop animate-fade-in flex items-center justify-center p-4" onClick={() => setRemId(null)}>
          <div className="w-full max-w-sm bg-slate-800/50 border border-slate-700/50 rounded-xl shadow-2xl animate-scale-in" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700/50">
              <h2 className="text-lg font-semibold text-white">Confirm Remediation</h2>
              <button onClick={() => setRemId(null)} className="text-slate-400 hover:text-white"><X className="w-5 h-5" /></button>
            </div>
            <div className="px-6 py-4">
              <p className="text-sm text-slate-400 mb-4">This will trigger automated remediation. Corrective network policies will be applied.</p>
              <div className="flex justify-end gap-2">
                <button onClick={() => setRemId(null)} className="px-4 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
                <button onClick={handleRemediate} disabled={remediating} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-yellow-600 text-white text-sm hover:bg-yellow-700 disabled:opacity-50 transition-colors">
                  {remediating ? <Loader2 className="w-4 h-4 animate-spin" /> : <Wrench className="w-4 h-4" />} Remediate
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

function SummaryCard({ label, value, sub, gradient, valueColor }: { label: string; value: number; sub?: string; gradient: string; valueColor?: string }) {
  return (
    <div className={`rounded-xl border border-slate-700/50 p-4 ${gradient}`}>
      <div className="text-xs text-slate-400 mb-1">{label}</div>
      <div className={`text-2xl font-bold ${valueColor ?? 'text-white'}`}>{value}</div>
      {sub && <div className="text-xs text-slate-400 mt-1">{sub}</div>}
    </div>
  );
}

export default Anomalies;
