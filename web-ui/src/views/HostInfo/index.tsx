import React, { useState, useEffect, useCallback } from 'react';
import { Monitor, Cpu, HardDrive, MemoryStick, RefreshCw, Loader2, Wifi, Globe } from 'lucide-react';
import { fetchHostInfo, HostInfo as HostInfoType } from '../../services/api';
import { usePageTitle } from '../../hooks/usePageTitle';

const HostInfoView: React.FC = () => {
  usePageTitle('Host Info');
  const [info, setInfo] = useState<HostInfoType | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetchHostInfo();
      setInfo(res.data);
    } catch {
      setError('Failed to fetch host info');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-cyan-500 to-cyan-700 flex items-center justify-center shadow-lg shadow-cyan-500/20">
            <Monitor className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">Host Information</h1>
            <p className="text-sm text-slate-400">System details and resource usage</p>
          </div>
        </div>
        <button onClick={fetchData} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} /> Refresh
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>
      )}

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      {info && (
        <div className="space-y-6">
          {/* System Info */}
          <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
            <h2 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
              <Monitor className="w-4 h-4 text-cyan-400" /> System
            </h2>
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
              <div><div className="text-xs text-slate-400 mb-1">Hostname</div><div className="text-sm font-medium text-white">{info.hostname}</div></div>
              <div><div className="text-xs text-slate-400 mb-1">OS</div><div className="text-sm font-medium text-white">{info.os}</div></div>
              <div><div className="text-xs text-slate-400 mb-1">Kernel</div><div className="text-sm font-medium text-white font-mono text-xs">{info.kernel}</div></div>
              <div><div className="text-xs text-slate-400 mb-1">Architecture</div><div className="text-sm font-medium text-white">{info.arch}</div></div>
            </div>
          </div>

          {/* Resource Usage */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ResourceCard
              icon={<Cpu className="w-5 h-5 text-white" />}
              label="CPU"
              gradient="from-blue-500 to-blue-700"
              glow="shadow-blue-500/20"
              usage={info.cpu_usage}
              detail={`${info.cpu_cores} cores @ ${info.cpu_model || 'Unknown'}`}
              color="#3b82f6"
            />
            <ResourceCard
              icon={<MemoryStick className="w-5 h-5 text-white" />}
              label="Memory"
              gradient="from-purple-500 to-purple-700"
              glow="shadow-purple-500/20"
              usage={info.memory_total_gb > 0 ? (info.memory_used_gb / info.memory_total_gb) * 100 : 0}
              detail={`${info.memory_used_gb.toFixed(1)} GB / ${info.memory_total_gb.toFixed(1)} GB`}
              color="#a855f7"
            />
            <ResourceCard
              icon={<HardDrive className="w-5 h-5 text-white" />}
              label="Disk"
              gradient="from-orange-500 to-orange-700"
              glow="shadow-orange-500/20"
              usage={info.disk_total_gb > 0 ? (info.disk_used_gb / info.disk_total_gb) * 100 : 0}
              detail={`${info.disk_used_gb.toFixed(1)} GB / ${info.disk_total_gb.toFixed(1)} GB`}
              color="#f97316"
            />
          </div>

          {/* Network Interfaces */}
          {info.network_interfaces && info.network_interfaces.length > 0 && (
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5">
              <h2 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
                <Globe className="w-4 h-4 text-green-400" /> Network Interfaces
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                {info.network_interfaces.map((iface) => (
                  <div key={iface.name} className="rounded-xl border border-slate-700/50 bg-slate-900/50 p-3">
                    <div className="flex items-center gap-2 mb-2">
                      <Wifi className="w-4 h-4 text-green-400" />
                      <span className="font-medium text-white text-sm">{iface.name}</span>
                      <span className={`text-xs px-1.5 py-0.5 rounded-full ${iface.status === 'up' ? 'bg-green-500/20 text-green-400' : 'bg-slate-500/20 text-slate-400'}`}>
                        {iface.status}
                      </span>
                    </div>
                    {iface.ip && <div className="text-xs text-slate-400 font-mono">{iface.ip}</div>}
                    {iface.mac && <div className="text-xs text-slate-500 font-mono">{iface.mac}</div>}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Load & Uptime */}
          <div className="grid grid-cols-2 gap-4">
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
              <div className="text-xs text-slate-400 mb-1">Load Average</div>
              <div className="text-lg font-bold text-white font-mono">
                {info.load_average ? info.load_average.join(' / ') : 'N/A'}
              </div>
            </div>
            <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4">
              <div className="text-xs text-slate-400 mb-1">Uptime</div>
              <div className="text-lg font-bold text-white">
                {info.uptime_seconds ? `${Math.floor(info.uptime_seconds / 3600)}h ${Math.floor((info.uptime_seconds % 3600) / 60)}m` : 'N/A'}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

function ResourceCard({ icon, label, gradient, glow, usage, detail, color }: {
  icon: React.ReactNode; label: string; gradient: string; glow: string;
  usage: number; detail: string; color: string;
}) {
  return (
    <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-5 card-glow transition-all hover:scale-[1.01]">
      <div className="flex items-center gap-3 mb-4">
        <div className={`w-10 h-10 rounded-lg bg-gradient-to-br ${gradient} flex items-center justify-center shadow-lg ${glow}`}>
          {icon}
        </div>
        <div>
          <div className="text-sm font-semibold text-white">{label}</div>
          <div className="text-xs text-slate-400">{detail}</div>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <div className="flex-1 h-2 rounded-full bg-slate-700 overflow-hidden">
          <div className="h-full rounded-full transition-all duration-300" style={{ width: `${Math.min(usage, 100)}%`, backgroundColor: color }} />
        </div>
        <span className="text-sm font-bold text-white w-12 text-right">{usage.toFixed(0)}%</span>
      </div>
    </div>
  );
}

export default HostInfoView;
