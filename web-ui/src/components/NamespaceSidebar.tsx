import React from 'react';
import { Box, Shield, ChevronRight } from 'lucide-react';

export interface NamespaceInfo {
  name: string;
  pods: number;
  endpoints: number;
  policies: number;
  status?: 'active' | 'warning' | 'error';
}

interface NamespaceSidebarProps {
  namespaces: NamespaceInfo[];
}

const STATUS_DOT: Record<string, string> = {
  active: 'bg-emerald-400',
  warning: 'bg-yellow-400',
  error: 'bg-red-400',
};

export const NamespaceSidebar: React.FC<NamespaceSidebarProps> = ({ namespaces }) => {
  return (
    <div className="bg-slate-800/50 rounded-xl border border-slate-700/50 p-5 h-full">
      <h3 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
        <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-orange-500 to-orange-700 shadow-lg shadow-orange-500/20 flex items-center justify-center">
          <Box className="w-4 h-4 text-white" />
        </div>
        Namespaces
        <span className="ml-auto text-[10px] font-medium px-2 py-0.5 rounded-full bg-slate-700/50 text-slate-400">
          {namespaces.length}
        </span>
      </h3>

      {namespaces.length === 0 ? (
        <div className="text-center py-8 text-slate-500 text-sm">
          No namespaces found
        </div>
      ) : (
        <div className="space-y-2 overflow-y-auto max-h-[360px] pr-1">
          {namespaces.map((ns) => {
            const dotColor = STATUS_DOT[ns.status || 'active'] || STATUS_DOT.active;
            return (
              <div
                key={ns.name}
                className="bg-slate-900/50 rounded-lg p-3 border border-slate-700/30 transition-all hover:border-slate-600/50 hover:bg-slate-900/70 cursor-pointer group"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className={`flex-shrink-0 w-2 h-2 rounded-full ${dotColor}`} />
                    <span className="text-sm font-semibold text-white truncate">
                      {ns.name}
                    </span>
                  </div>
                  <ChevronRight className="w-3.5 h-3.5 text-slate-600 group-hover:text-slate-400 transition-colors flex-shrink-0" />
                </div>
                <div className="grid grid-cols-3 gap-2">
                  <div className="flex flex-col items-center bg-slate-800/60 rounded-md py-1.5 px-1">
                    <span className="text-xs font-bold text-blue-400">{ns.pods}</span>
                    <span className="text-[10px] text-slate-500">Pods</span>
                  </div>
                  <div className="flex flex-col items-center bg-slate-800/60 rounded-md py-1.5 px-1">
                    <span className="text-xs font-bold text-cyan-400">{ns.endpoints}</span>
                    <span className="text-[10px] text-slate-500">Endpoints</span>
                  </div>
                  <div className="flex flex-col items-center bg-slate-800/60 rounded-md py-1.5 px-1">
                    <span className="text-xs font-bold text-purple-400">{ns.policies}</span>
                    <span className="text-[10px] text-slate-500">
                      <Shield className="w-3 h-3 inline-block" />
                    </span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default NamespaceSidebar;
