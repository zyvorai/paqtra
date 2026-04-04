import React, { useState, useEffect, useCallback } from 'react';
import { Siren, RefreshCw, Loader2, ChevronDown, ChevronRight, Clock } from 'lucide-react';
import { fetchIncidents, Incident } from '../../services/api';
import { isAxiosError } from 'axios';
import { usePageTitle } from '../../hooks/usePageTitle';
import { formatRelativeTime } from '../../utils/formatters';

const SEV_BADGE: Record<string, string> = { critical: 'bg-red-500/15 text-red-400 border-red-500/30', high: 'bg-orange-500/15 text-orange-400 border-orange-500/30', medium: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/30', low: 'bg-blue-500/15 text-blue-400 border-blue-500/30' };
const STATUS_BADGE: Record<string, string> = { active: 'bg-red-500/15 text-red-400 border-red-500/30', investigating: 'bg-orange-500/15 text-orange-400 border-orange-500/30', resolved: 'bg-green-500/15 text-green-400 border-green-500/30' };

const IncidentTimeline: React.FC = () => {
  usePageTitle('Incident Timeline');
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const fetchData = useCallback(async () => {
    setLoading(true); setError(null);
    try { const res = await fetchIncidents(); setIncidents(res.data.incidents ?? []); }
    catch (err) { setError(isAxiosError(err) ? err.response?.data?.message ?? err.message : 'Failed to load incidents'); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { fetchData(); }, [fetchData]);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const activeCount = incidents.filter((i) => i.status !== 'resolved').length;
  const resolvedCount = incidents.filter((i) => i.status === 'resolved').length;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-red-500 to-red-700 flex items-center justify-center shadow-lg shadow-red-500/20">
              <Siren className="w-5 h-5 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-white">Incident Timeline</h1>
          </div>
          <p className="text-sm text-slate-400 mt-1">Track and review incident history</p>
        </div>
        <button onClick={fetchData} disabled={loading} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-700/50 text-sm text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
          <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {error && <div className="mb-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-blue card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Total Incidents</div>
          <div className="text-2xl font-bold text-white">{incidents.length}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-red card-glow transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Active</div>
          <div className="text-2xl font-bold text-red-400">{activeCount}</div>
        </div>
        <div className="rounded-xl border border-slate-700/50 p-4 stat-card-green card-glow-green transition-all hover:scale-[1.02]">
          <div className="text-xs text-slate-400 mb-1">Resolved</div>
          <div className="text-2xl font-bold text-green-400">{resolvedCount}</div>
        </div>
      </div>

      {loading && <Loader2 className="w-6 h-6 animate-spin text-blue-400 mx-auto my-8" />}

      <div className="relative">
        {incidents.length > 1 && <div className="absolute left-5 top-0 bottom-0 w-0.5 bg-slate-700" />}
        <div className="space-y-4">
          {incidents.map((inc) => {
            const isOpen = expanded.has(inc.id);
            return (
              <div key={inc.id} className="relative pl-12">
                <div className="absolute left-[14px] top-5 w-3 h-3 rounded-full bg-blue-600 border-2 border-slate-800 z-10" />
                <div
                  className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-4 cursor-pointer card-glow transition-all hover:scale-[1.01]"
                  onClick={() => toggle(inc.id)}
                >
                  <div className="flex items-center gap-2 mb-2">
                    {isOpen ? <ChevronDown className="w-4 h-4 text-slate-400" /> : <ChevronRight className="w-4 h-4 text-slate-400" />}
                    <span className={`px-2 py-0.5 rounded-full text-xs border ${SEV_BADGE[inc.severity] ?? 'bg-slate-500/15 text-slate-400 border-slate-500/30'}`}>{inc.severity}</span>
                    <span className={`px-2 py-0.5 rounded-full text-xs border ${STATUS_BADGE[inc.status] ?? 'bg-slate-500/15 text-slate-400 border-slate-500/30'}`}>{inc.status}</span>
                    <span className="font-medium text-white">{inc.title}</span>
                    <span className="ml-auto flex items-center gap-1 text-xs text-slate-400">
                      <Clock className="w-3 h-3" /> {formatRelativeTime(inc.started_at)}
                    </span>
                  </div>
                  {inc.duration && <div className="text-xs text-slate-400">Duration: {inc.duration}</div>}

                  {isOpen && (
                    <div className="mt-4 space-y-3 border-t border-slate-700/50 pt-4">
                      {inc.affected_services && inc.affected_services.length > 0 && (
                        <div>
                          <div className="text-xs font-semibold text-slate-400 mb-1">Affected Services</div>
                          <div className="flex flex-wrap gap-2">
                            {inc.affected_services.map((svc) => (
                              <span key={svc} className="px-2 py-0.5 rounded-full text-xs bg-slate-700/50 text-slate-300 border border-slate-600/50">{svc}</span>
                            ))}
                          </div>
                        </div>
                      )}
                      {inc.root_cause && (
                        <div>
                          <div className="text-xs font-semibold text-slate-400 mb-1">Root Cause</div>
                          <div className="text-sm text-slate-300">{inc.root_cause}</div>
                        </div>
                      )}
                      {inc.timeline && inc.timeline.length > 0 && (
                        <div>
                          <div className="text-xs font-semibold text-slate-400 mb-2">Timeline Events</div>
                          <div className="space-y-2">
                            {inc.timeline.map((evt, idx) => (
                              <div key={idx} className="flex items-start gap-3 text-sm">
                                <span className="text-xs text-slate-500 font-mono whitespace-nowrap mt-0.5">{evt.time}</span>
                                <span className="text-slate-300">{evt.event}</span>
                                {evt.actor && <span className="text-xs text-slate-500 ml-auto">{evt.actor}</span>}
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

export default IncidentTimeline;
