import React, { useState, useCallback } from 'react';
import { Loader2, Play, ArrowDownRight } from 'lucide-react';
import { fetchPacketDrops, analyzeDrops, PacketDrop } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

const REASON_BADGE: Record<string, string> = {
  POLICY_DENIED: 'bg-red-500/15 text-red-400 border-red-500/30',
  CT_MAP_INSERTION_FAILED: 'bg-orange-500/15 text-orange-400 border-orange-500/30',
  NO_TUNNEL_ENDPOINT: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30',
  INVALID_SOURCE_MAC: 'bg-purple-500/15 text-purple-400 border-purple-500/30',
};

const RootCause: React.FC = () => {
  usePageTitle('Root Cause');
  const [drops, setDrops] = useState<PacketDrop[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<Record<string, unknown> | null>(null);
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  const fetchData = useCallback(async () => {
    setError(null);
    try { setDrops((await fetchPacketDrops()).data.drops ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(fetchData, 30000, autoRefreshOn);

  const handleAnalyze = async () => {
    setAnalyzing(true); setError(null);
    try { setAnalysis((await analyzeDrops({})).data.analysis as Record<string, unknown>); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Analysis failed'); }
    finally { setAnalyzing(false); }
  };

  const totalDrops = drops.reduce((a, d) => a + d.count, 0);
  const uniqueReasons = [...new Set(drops.map((d) => d.drop_reason))];

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-blue-700 flex items-center justify-center shadow-lg shadow-blue-500/20"><ArrowDownRight className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Root Cause Analysis</h1></div>
          <p className="text-sm text-slate-400 mt-1">Packet drop analysis with one-click remediation</p>
        </div>
        <div className="flex items-center gap-2">
          {drops.length > 0 && <ExportButton data={drops as unknown as Record<string, unknown>[]} filename="packet-drops" />}
          <button onClick={handleAnalyze} disabled={analyzing} className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
            {analyzing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />} Analyze
          </button>
          <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
        </div>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      {/* Summary */}
      <div className="grid grid-cols-2 lg:grid-cols-3 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Drops</div>
          <div className="text-2xl font-bold text-red-400">{totalDrops}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Unique Reasons</div>
          <div className="text-2xl font-bold text-white">{uniqueReasons.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Affected Sources</div>
          <div className="text-2xl font-bold text-white">{drops.length}</div>
        </div>
      </div>

      {/* Analysis result */}
      {analysis && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 mb-6 animate-scale-in">
          <h2 className="text-sm font-semibold text-white mb-3">Analysis Result</h2>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
            {Object.entries(analysis).map(([k, v]) => (
              <div key={k}><span className="text-slate-400">{k.replace(/_/g, ' ')}: </span><span className="text-white font-medium">{String(v)}</span></div>
            ))}
          </div>
        </div>
      )}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {/* Drop entries */}
      {!loading && drops.length === 0 && (
        <div className="text-center py-12 text-slate-400">
          <ArrowDownRight className="w-12 h-12 mx-auto mb-3 text-green-400" />
          <div className="font-medium text-white">No packet drops detected</div>
          <div className="text-sm">All traffic is flowing normally.</div>
        </div>
      )}
      <div className="space-y-3">
        {drops.map((d) => (
          <div key={d.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 card-glow transition-all hover:scale-[1.01]">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-2">
                  <span className={`px-2 py-0.5 rounded-full text-xs border ${REASON_BADGE[d.drop_reason] ?? 'bg-slate-900/50 text-slate-400 border-slate-700/50'}`}>{d.drop_reason}</span>
                  <span className="text-xs text-slate-400">{d.protocol}</span>
                  <span className="text-xs text-red-400 font-medium">{d.count}x</span>
                </div>
                <div className="text-sm mb-1">
                  <span className="text-slate-400">Source: </span><span className="text-white font-mono">{d.source}</span>
                  <span className="text-slate-400 mx-2">&rarr;</span>
                  <span className="text-white font-mono">{d.destination}</span>
                </div>
                <div className="text-sm"><span className="text-slate-400">Root Cause: </span><span className="text-white">{d.root_cause}</span></div>
              </div>
              <div className="p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-sm max-w-xs">
                <div className="text-xs text-green-400 font-medium mb-1">Remediation</div>
                <div className="text-green-300">{d.remediation}</div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default RootCause;
