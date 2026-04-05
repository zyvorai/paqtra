import React, { useState, useEffect, useCallback } from 'react';
import { Radio, RefreshCw, Loader2, Play, Square, Plus, HardDrive } from 'lucide-react';
import { fetchCaptureSessions, startCapture, stopCapture, CaptureSession } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { useAutoDismiss } from '../../hooks/useAutoDismiss';

const STATUS_BADGE: Record<string, string> = { capturing: 'bg-red-500/15 text-red-400 border-red-500/30', completed: 'bg-green-500/15 text-green-400 border-green-500/30' };
function formatBytes(b: number): string { return b < 1048576 ? `${(b / 1024).toFixed(1)} KB` : `${(b / 1048576).toFixed(1)} MB`; }

const PacketCapture: React.FC = () => {
  usePageTitle('Packet Capture');
  const [sessions, setSessions] = useState<CaptureSession[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useAutoDismiss<string | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [newName, setNewName] = useState('');
  const [newPod, setNewPod] = useState('');
  const [newNs, setNewNs] = useState('default');
  const [newFilter, setNewFilter] = useState('');
  const [starting, setStarting] = useState(false);
  const [stopping, setStopping] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { setSessions((await fetchCaptureSessions()).data.sessions ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const handleStart = async () => {
    if (!newName || !newPod) return;
    setStarting(true); setError(null);
    try { await startCapture({ name: newName, target_pod: newPod, namespace: newNs, filter: newFilter || undefined }); setSuccess('Capture started'); setShowNew(false); setNewName(''); setNewPod(''); fetchData(); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed'); }
    finally { setStarting(false); }
  };

  const handleStop = async (id: string) => {
    setStopping(id); setError(null);
    try { await stopCapture(id); setSuccess('Capture stopped'); fetchData(); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to stop capture'); }
    finally { setStopping(null); }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3"><div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500 to-cyan-700 flex items-center justify-center shadow-lg shadow-cyan-500/20"><Radio className="w-5 h-5 text-white" /></div><h1 className="text-2xl font-bold text-white">Packet Capture</h1></div>
          <p className="text-sm text-slate-400 mt-1">tcpdump-style packet capture for debugging</p>
        </div>
        <div className="flex gap-2">
          <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors"><RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /></button>
          <button onClick={() => setShowNew(true)} className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 transition-colors"><Plus className="w-4 h-4" /> New Capture</button>
        </div>
      </div>
      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}
      {success && <div className="mb-4 p-3 rounded-lg bg-green-500/10 border border-green-500/30 text-green-400 text-sm">{success}</div>}
      {showNew && (
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 mb-6 animate-scale-in">
          <h2 className="text-sm font-semibold text-white mb-4 flex items-center gap-2"><Radio className="w-4 h-4 text-cyan-400" /> Start New Capture</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
            <input value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="Capture name" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input value={newPod} onChange={(e) => setNewPod(e.target.value)} placeholder="Target pod" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input value={newNs} onChange={(e) => setNewNs(e.target.value)} placeholder="Namespace" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input value={newFilter} onChange={(e) => setNewFilter(e.target.value)} placeholder="BPF filter (optional)" className="px-3 py-2 rounded-lg bg-slate-900/50 border border-slate-700/50 text-sm text-white font-mono focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <div className="flex gap-2">
              <button onClick={handleStart} disabled={starting || !newName || !newPod} className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-red-600 text-white text-sm hover:bg-red-700 disabled:opacity-50 transition-colors">
                {starting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Play className="w-4 h-4" />} Capture
              </button>
              <button onClick={() => setShowNew(false)} className="px-3 py-2 rounded-lg border border-slate-700/50 text-sm hover:bg-slate-700/30 transition-colors">Cancel</button>
            </div>
          </div>
        </div>
      )}
      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {sessions.map((s) => (
          <div key={s.id} className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
            <div className="flex items-center justify-between mb-3">
              <span className="font-semibold text-white">{s.name}</span>
              <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[s.status] ?? ''}`}>
                {s.status === 'capturing' && <span className="animate-pulse-dot w-1.5 h-1.5 rounded-full bg-red-400 inline-block mr-1" />}{s.status}
              </span>
            </div>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div><span className="text-slate-400">Pod: </span><span className="text-white font-mono">{s.target_pod}</span></div>
              <div><span className="text-slate-400">Namespace: </span><span className="text-white">{s.namespace}</span></div>
              <div><span className="text-slate-400">Interface: </span><span className="text-white">{s.interface_name}</span></div>
              <div><span className="text-slate-400">Filter: </span><span className="text-white font-mono">{s.filter || 'none'}</span></div>
              <div><span className="text-slate-400">Packets: </span><span className="text-white font-medium">{(s.packet_count ?? 0).toLocaleString()}</span></div>
              <div className="flex items-center gap-1"><HardDrive className="w-3 h-3 text-slate-400" /><span className="text-white">{formatBytes(s.size_bytes ?? s.size ?? 0)}</span></div>
            </div>
            {s.status === 'capturing' && (
              <button onClick={() => handleStop(s.id)} disabled={stopping === s.id} className="mt-3 w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded-lg border border-red-500/30 text-red-400 text-sm hover:bg-red-500/10 disabled:opacity-50 transition-colors">{stopping === s.id ? <Loader2 className="w-4 h-4 animate-spin" /> : <Square className="w-4 h-4" />} Stop</button>
            )}
          </div>
        ))}
        {!loading && sessions.length === 0 && <div className="col-span-full text-center py-12 text-slate-400">No capture sessions. Start one to debug network issues.</div>}
      </div>
    </div>
  );
};

export default PacketCapture;
