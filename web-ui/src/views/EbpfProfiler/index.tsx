import React, { useState, useEffect } from 'react';
import { Cpu, RefreshCw, Loader2, Zap, Database } from 'lucide-react';
import { fetchEbpfPrograms, fetchEbpfMaps, EbpfProgram, EbpfMapInfo } from '../../services/api';
import { isAxiosError } from 'axios';
import { formatCount } from '../../utils/formatters';
import { usePageTitle } from '../../hooks/usePageTitle';

const EbpfProfiler: React.FC = () => {
  usePageTitle('eBPF Profiler');
  const [programs, setPrograms] = useState<EbpfProgram[]>([]);
  const [maps, setMaps] = useState<EbpfMapInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'programs' | 'maps'>('programs');

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [progRes, mapRes] = await Promise.all([fetchEbpfPrograms(), fetchEbpfMaps()]);
      setPrograms(progRes.data.programs ?? []);
      setMaps(mapRes.data.maps ?? []);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load eBPF data');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadData(); }, []);

  const totalRuns = programs.reduce((sum, p) => sum + p.run_count, 0);
  const totalMapEntries = maps.reduce((sum, m) => sum + m.current_entries, 0);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-orange-500 to-orange-700 flex items-center justify-center shadow-lg shadow-orange-500/20">
              <Cpu className="w-5 h-5 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-white">eBPF Profiler</h1>
          </div>
          <p className="text-sm text-slate-400 mt-1">Inspect loaded eBPF programs and maps</p>
        </div>
        <button onClick={loadData} disabled={loading}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 disabled:opacity-50 transition-colors">
          {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-4 gap-3 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Programs</div>
          <div className="text-2xl font-bold text-blue-400">{formatCount(programs.length)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-purple card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Maps</div>
          <div className="text-2xl font-bold text-purple-400">{formatCount(maps.length)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-1 text-xs text-slate-400 mb-1"><Zap className="w-3 h-3" />Total Runs</div>
          <div className="text-2xl font-bold text-green-400">{formatCount(totalRuns)}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-orange card-glow transition-all hover:scale-[1.02]">
          <div className="flex items-center gap-1 text-xs text-slate-400 mb-1"><Database className="w-3 h-3" />Total Map Entries</div>
          <div className="text-2xl font-bold text-orange-400">{formatCount(totalMapEntries)}</div>
        </div>
      </div>

      <div className="p-1 rounded-lg bg-slate-900/50 inline-flex mb-6">
        <button onClick={() => setActiveTab('programs')}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${activeTab === 'programs' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}>
          Programs
        </button>
        <button onClick={() => setActiveTab('maps')}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${activeTab === 'maps' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}>
          Maps
        </button>
      </div>

      {activeTab === 'programs' && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-slate-900/50">
              <tr className="text-left text-slate-400">
                <th className="px-4 py-3 font-medium">Name</th>
                <th className="px-4 py-3 font-medium">Type</th>
                <th className="px-4 py-3 font-medium">Attached To</th>
                <th className="px-4 py-3 font-medium text-right">Run Count</th>
                <th className="px-4 py-3 font-medium text-right">Map Count</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/50">
              {programs.map((p) => (
                <tr key={p.id} className="table-row-hover">
                  <td className="px-4 py-3 text-white font-medium">{p.name}</td>
                  <td className="px-4 py-3 text-slate-400">{p.type}</td>
                  <td className="px-4 py-3 text-slate-400">{p.attached_to || p.attach_point || '-'}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(p.run_count)}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{p.map_count ?? p.map_ids.length}</td>
                </tr>
              ))}
              {programs.length === 0 && !loading && (
                <tr><td colSpan={5} className="px-4 py-8 text-center text-slate-400">No eBPF programs found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {activeTab === 'maps' && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-slate-900/50">
              <tr className="text-left text-slate-400">
                <th className="px-4 py-3 font-medium">Name</th>
                <th className="px-4 py-3 font-medium">Type</th>
                <th className="px-4 py-3 font-medium text-right">Key/Value Size</th>
                <th className="px-4 py-3 font-medium text-right">Max Entries</th>
                <th className="px-4 py-3 font-medium text-right">Current Entries</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/50">
              {maps.map((m) => (
                <tr key={m.id} className="table-row-hover">
                  <td className="px-4 py-3 text-white font-medium">{m.name}</td>
                  <td className="px-4 py-3 text-slate-400">{m.type}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{m.key_size} / {m.value_size}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(m.max_entries)}</td>
                  <td className="px-4 py-3 text-right text-slate-300">{formatCount(m.current_entries)}</td>
                </tr>
              ))}
              {maps.length === 0 && !loading && (
                <tr><td colSpan={5} className="px-4 py-8 text-center text-slate-400">No eBPF maps found</td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default EbpfProfiler;
