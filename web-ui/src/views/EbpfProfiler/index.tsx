import React, { useState, useCallback } from 'react';
import { Cpu, Zap, Database, Search } from 'lucide-react';
import { fetchEbpfPrograms, fetchEbpfMaps, fetchRealEbpfPrograms, fetchRealEbpfMaps, fetchEbpfMapEntries, EbpfProgram, EbpfMapInfo } from '../../services/api';
import { isAxiosError } from 'axios';
import { formatCount } from '../../utils/formatters';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoRefresh } from '../../hooks/useAutoRefresh';
import DataFreshness from '../../components/DataFreshness';
import ExportButton from '../../components/ExportButton';

interface MapEntry {
  key: string;
  value: string;
  [k: string]: unknown;
}

const EbpfProfiler: React.FC = () => {
  usePageTitle('eBPF Profiler');
  const [programs, setPrograms] = useState<EbpfProgram[]>([]);
  const [maps, setMaps] = useState<EbpfMapInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'programs' | 'maps' | 'explorer'>('programs');
  const [autoRefreshOn, setAutoRefreshOn] = useState(true);

  // Map Explorer state
  const [selectedMapId, setSelectedMapId] = useState<number | null>(null);
  const [mapEntries, setMapEntries] = useState<MapEntry[]>([]);
  const [mapEntriesLoading, setMapEntriesLoading] = useState(false);
  const [mapEntriesError, setMapEntriesError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setError(null);
    try {
      // Try real API first, fall back to module API
      let progData: EbpfProgram[] = [];
      let mapData: EbpfMapInfo[] = [];

      try {
        const [progRes, mapRes] = await Promise.all([fetchRealEbpfPrograms(), fetchRealEbpfMaps()]);
        progData = (progRes.data.programs ?? []) as EbpfProgram[];
        mapData = (mapRes.data.maps ?? []) as EbpfMapInfo[];
      } catch {
        // Fallback to module API
        const [progRes, mapRes] = await Promise.all([fetchEbpfPrograms(), fetchEbpfMaps()]);
        progData = progRes.data.programs ?? [];
        mapData = mapRes.data.maps ?? [];
      }

      setPrograms(progData);
      setMaps(mapData);
    } catch (err) {
      setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load eBPF data');
    }
  }, []);

  const { lastUpdated, refreshing: loading, manualRefresh } = useAutoRefresh(loadData, 30000, autoRefreshOn);

  const totalRuns = programs.reduce((sum, p) => sum + (p.run_count ?? 0), 0);
  const totalMapEntries = maps.reduce((sum, m) => sum + (m.current_entries ?? 0), 0);

  const loadMapEntries = useCallback(async (mapId: number) => {
    setSelectedMapId(mapId);
    setMapEntriesLoading(true);
    setMapEntriesError(null);
    try {
      const res = await fetchEbpfMapEntries(mapId);
      setMapEntries((res.data as { entries?: MapEntry[] }).entries ?? (res.data as unknown as MapEntry[]) ?? []);
    } catch (err) {
      setMapEntriesError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load map entries');
      setMapEntries([]);
    } finally {
      setMapEntriesLoading(false);
    }
  }, []);

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
        <DataFreshness lastUpdated={lastUpdated} onRefresh={manualRefresh} refreshing={loading} autoRefresh={autoRefreshOn} onAutoRefreshToggle={() => setAutoRefreshOn((v) => !v)} intervalSecs={30} />
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
        <button onClick={() => setActiveTab('explorer')}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${activeTab === 'explorer' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}>
          <span className="flex items-center gap-1.5"><Search className="w-3.5 h-3.5" />Map Explorer</span>
        </button>
      </div>

      {activeTab === 'programs' && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
          {programs.length > 0 && (
            <div className="px-4 py-3 border-b border-slate-700/50 flex justify-end">
              <ExportButton data={programs as unknown as Record<string, unknown>[]} filename="ebpf-programs" />
            </div>
          )}
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
                  <td className="px-4 py-3 text-right text-slate-300">{p.map_count ?? p.map_ids?.length ?? 0}</td>
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
          {maps.length > 0 && (
            <div className="px-4 py-3 border-b border-slate-700/50 flex justify-end">
              <ExportButton data={maps as unknown as Record<string, unknown>[]} filename="ebpf-maps" />
            </div>
          )}
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

      {activeTab === 'explorer' && (
        <div className="grid grid-cols-3 gap-6">
          {/* Map list */}
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
            <div className="px-4 py-3 border-b border-slate-700/50">
              <h3 className="text-sm font-medium text-white">Select a Map</h3>
            </div>
            <div className="max-h-[500px] overflow-y-auto">
              {maps.map((m) => (
                <button
                  key={m.id}
                  onClick={() => loadMapEntries(Number(m.id))}
                  className={`w-full text-left px-4 py-3 border-b border-slate-700/30 text-sm transition-colors hover:bg-slate-700/50 ${
                    selectedMapId === Number(m.id) ? 'bg-blue-600/20 text-blue-400' : 'text-slate-300'
                  }`}
                >
                  <div className="font-medium">{m.name}</div>
                  <div className="text-xs text-slate-500 mt-0.5">{m.type} - {formatCount(m.current_entries)} entries</div>
                </button>
              ))}
              {maps.length === 0 && (
                <div className="px-4 py-8 text-center text-slate-400 text-sm">No maps available</div>
              )}
            </div>
          </div>

          {/* Entries */}
          <div className="col-span-2 rounded-xl border border-slate-700/50 bg-slate-800/50 overflow-hidden">
            <div className="px-4 py-3 border-b border-slate-700/50">
              <h3 className="text-sm font-medium text-white">
                {selectedMapId !== null ? `Map Entries (ID: ${selectedMapId})` : 'Map Entries'}
              </h3>
            </div>
            {mapEntriesError && (
              <div className="m-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{mapEntriesError}</div>
            )}
            {mapEntriesLoading && (
              <div className="px-4 py-8 text-center text-slate-400 text-sm">Loading entries...</div>
            )}
            {!mapEntriesLoading && selectedMapId === null && (
              <div className="px-4 py-8 text-center text-slate-400 text-sm">Click a map to view its entries</div>
            )}
            {!mapEntriesLoading && selectedMapId !== null && mapEntries.length === 0 && !mapEntriesError && (
              <div className="px-4 py-8 text-center text-slate-400 text-sm">No entries in this map</div>
            )}
            {!mapEntriesLoading && mapEntries.length > 0 && (
              <div className="overflow-x-auto max-h-[450px] overflow-y-auto">
                <table className="w-full text-sm">
                  <thead className="bg-slate-900/50 sticky top-0">
                    <tr className="text-left text-slate-400">
                      <th className="px-4 py-3 font-medium">Key (hex)</th>
                      <th className="px-4 py-3 font-medium">Value (hex)</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-700/50">
                    {mapEntries.map((entry, i) => (
                      <tr key={i} className="table-row-hover">
                        <td className="px-4 py-3 text-cyan-400 font-mono text-xs break-all">{entry.key}</td>
                        <td className="px-4 py-3 text-slate-300 font-mono text-xs break-all">{entry.value}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default EbpfProfiler;
