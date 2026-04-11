import React, { useState, useCallback, useRef, useEffect } from 'react';
import { NavLink, useNavigate, useLocation, Outlet } from 'react-router-dom';
import {
  LayoutDashboard, Network, GitBranch, Shield, Bug, ShieldCheck, Wand2,
  FlaskConical, Bird, Settings, Search, Sun, Moon, LogOut, Menu, X,
  Bell, CircleDot, Server, Film, HeartPulse, ArrowDownRight, Globe, Grid3X3,
  Workflow, ShieldAlert, Cpu, BarChart3, Monitor, FileCode, Stethoscope,
  ScrollText, BellRing, Map, Radio, Globe2, Fingerprint, Link2, Route, Gauge,
  DollarSign, TrendingUp, Lock, Scale, Timer, Copy,
  Users, Cable, Wrench, Activity, FileEdit, Download, Target, Siren,
  History, ServerOff, Unplug, Hexagon, ChevronDown, User,
  Database, AlertTriangle,
} from 'lucide-react';
import { useThemeStore } from '../stores/themeStore';
import { useAuthStore } from '../stores/authStore';
import { useKeyboardShortcuts, shortcuts } from '../hooks/useKeyboardShortcuts';
import GlobalSearch from './GlobalSearch';

interface NavGroup {
  label: string;
  items: { label: string; path: string; icon: React.ComponentType<{ className?: string }> }[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    label: 'Overview',
    items: [
      { label: 'Dashboard', path: '/', icon: LayoutDashboard },
      { label: 'Events', path: '/events', icon: Bell },
      { label: 'Nodes', path: '/nodes', icon: Server },
      { label: 'Endpoints', path: '/endpoints', icon: CircleDot },
      { label: 'Host Info', path: '/host', icon: Monitor },
      { label: 'Cluster Health', path: '/clusterhealth', icon: HeartPulse },
      { label: 'Cilium Status', path: '/cilium-status', icon: Activity },
    ],
  },
  {
    label: 'Observability',
    items: [
      { label: 'Flows', path: '/flows', icon: Network },
      { label: 'Topology', path: '/topology', icon: GitBranch },
      { label: 'Service Map', path: '/servicemap', icon: Map },
      { label: 'Heatmap', path: '/heatmap', icon: Grid3X3 },
      { label: 'Dependencies', path: '/dependencies', icon: Workflow },
      { label: 'Latency', path: '/latency', icon: Timer },
      { label: 'Flow Export', path: '/flow-export', icon: Download },
      { label: 'SLOs', path: '/slo', icon: Target },
      { label: 'DNS Monitor', path: '/dns', icon: Globe2 },
      { label: 'Bandwidth', path: '/bandwidth', icon: Gauge },
      { label: 'Interfaces', path: '/interfaces', icon: Cable },
      { label: 'Metrics', path: '/metrics', icon: BarChart3 },
    ],
  },
  {
    label: 'Security',
    items: [
      { label: 'Policies', path: '/policies', icon: Shield },
      { label: 'Templates', path: '/templates', icon: FileCode },
      { label: 'Policy Editor', path: '/policy-editor', icon: FileEdit },
      { label: 'Rule Builder', path: '/rule-builder', icon: Wand2 },
      { label: 'Pod Security', path: '/pod-security', icon: ShieldCheck },
      { label: 'Anomalies', path: '/anomalies', icon: Bug },
      { label: 'Security', path: '/security', icon: ShieldAlert },
      { label: 'Encryption', path: '/encryption', icon: Lock },
      { label: 'WireGuard', path: '/wireguard', icon: Shield },
      { label: 'Identities', path: '/identities', icon: Fingerprint },
      { label: 'RBAC', path: '/rbac', icon: Users },
      { label: 'Compliance', path: '/compliance', icon: ShieldCheck },
      { label: 'Audit Log', path: '/audit', icon: ScrollText },
      { label: 'Alerts', path: '/alerts', icon: BellRing },
      { label: 'Incidents', path: '/incidents', icon: Siren },
      { label: 'Change Log', path: '/changelog', icon: History },
    ],
  },
  {
    label: 'Intelligence',
    items: [
      { label: 'AutoPolicy', path: '/autopolicy', icon: Wand2 },
      { label: 'Healer', path: '/healer', icon: HeartPulse },
      { label: 'Root Cause', path: '/rootcause', icon: ArrowDownRight },
      { label: 'Diagnostics', path: '/diagnostics', icon: Stethoscope },
      { label: 'Troubleshoot', path: '/troubleshoot', icon: Wrench },
      { label: 'Forecasting', path: '/forecast', icon: TrendingUp },
    ],
  },
  {
    label: 'Operations',
    items: [
      { label: 'Chaos', path: '/chaos', icon: FlaskConical },
      { label: 'Canary', path: '/canary', icon: Bird },
      { label: 'Replay', path: '/replay', icon: Film },
      { label: 'Capture', path: '/capture', icon: Radio },
      { label: 'Mirroring', path: '/mirror', icon: Copy },
      { label: 'MultiCluster', path: '/multicluster', icon: Globe },
      { label: 'Cluster Mesh', path: '/clustermesh', icon: Link2 },
      { label: 'BGP Peering', path: '/bgp', icon: Route },
      { label: 'Node Drain', path: '/node-drain', icon: ServerOff },
      { label: 'eBPF', path: '/ebpf', icon: Cpu },
    ],
  },
  {
    label: 'eBPF Data',
    items: [
      { label: 'Conntrack Table', path: '/conntrack', icon: Database },
      { label: 'Policy Map', path: '/policy-map', icon: Shield },
      { label: 'IP Cache', path: '/ipcache', icon: Globe },
      { label: 'LB Map', path: '/lb-map', icon: Scale },
      { label: 'Drop Analytics', path: '/drops', icon: AlertTriangle },
    ],
  },
  {
    label: 'Networking',
    items: [
      { label: 'Load Balancer', path: '/loadbalancer', icon: Scale },
      { label: 'Ingress', path: '/ingress', icon: Globe },
      { label: 'Egress GW', path: '/egress', icon: LogOut },
      { label: 'Service Mesh', path: '/service-mesh', icon: Hexagon },
      { label: 'KPR', path: '/kpr', icon: Unplug },
      { label: 'IPAM', path: '/ipam', icon: Network },
      { label: 'Cost Analytics', path: '/costs', icon: DollarSign },
      { label: 'Settings', path: '/settings', icon: Settings },
    ],
  },
];

const MainLayout: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { isDark, toggle: toggleTheme } = useThemeStore();
  const { username, logout } = useAuthStore();
  const [searchOpen, setSearchOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  const [wsConnected, setWsConnected] = useState(false);
  const dropdownTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleToggleSearch = useCallback(() => setSearchOpen((v) => !v), []);
  const handleToggleHelp = useCallback(() => setHelpOpen((v) => !v), []);
  const handleNavigate = useCallback((path: string) => navigate(path), [navigate]);

  useKeyboardShortcuts({
    onToggleSearch: handleToggleSearch,
    onToggleHelp: handleToggleHelp,
    navigate: handleNavigate,
  });

  // Check server connectivity via health endpoint
  useEffect(() => {
    const checkHealth = () => {
      fetch('/api/v1/health')
        .then((res) => setWsConnected(res.ok))
        .catch(() => setWsConnected(false));
    };
    checkHealth();
    const interval = setInterval(checkHealth, 30000);
    return () => clearInterval(interval);
  }, []);

  // Close dropdown on outside click — only attach when a dropdown is open
  useEffect(() => {
    if (!openGroup) return;
    const handler = () => setOpenGroup(null);
    document.addEventListener('click', handler);
    return () => document.removeEventListener('click', handler);
  }, [openGroup]);

  // Close dropdown on route change
  useEffect(() => { setOpenGroup(null); setMobileOpen(false); }, [location.pathname]);

  const handleDropdownEnter = (label: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (dropdownTimeoutRef.current) {
      clearTimeout(dropdownTimeoutRef.current);
      dropdownTimeoutRef.current = null;
    }
    setOpenGroup(label);
  };

  const handleDropdownLeave = () => {
    dropdownTimeoutRef.current = setTimeout(() => {
      setOpenGroup(null);
    }, 150);
  };

  const isGroupActive = (group: NavGroup) =>
    group.items.some((item) => item.path === '/' ? location.pathname === '/' : location.pathname.startsWith(item.path));

  return (
    <div className="h-screen flex flex-col bg-slate-950">
      {/* ── Top Navbar ─────────────────────────────────── */}
      <header className="sticky top-0 z-50 navbar-gradient border-b border-slate-700/50 flex-shrink-0">
        <div className="flex items-center h-14 px-4">
          {/* Logo */}
          <NavLink to="/" className="flex items-center gap-2 mr-8 flex-shrink-0">
            <h1 className="text-xl font-bold text-gradient-blue">
              Cilium Vision
            </h1>
          </NavLink>

          {/* Desktop Nav Groups */}
          <nav className="hidden md:flex items-center gap-1 flex-1">
            {NAV_GROUPS.map((group) => (
              <div
                key={group.label}
                className="relative"
                onMouseEnter={(e) => handleDropdownEnter(group.label, e)}
                onMouseLeave={handleDropdownLeave}
              >
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setOpenGroup(openGroup === group.label ? null : group.label);
                  }}
                  className={`flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                    isGroupActive(group)
                      ? 'bg-blue-600/20 text-blue-400'
                      : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/50'
                  }`}
                >
                  {group.label}
                  <ChevronDown className="h-3.5 w-3.5" />
                </button>

                {/* Dropdown Menu */}
                {openGroup === group.label && (
                  <div className="absolute top-full left-0 mt-1 bg-slate-800 border border-slate-700 rounded-xl shadow-2xl p-2 min-w-[200px] z-50">
                    {group.items.map((item) => (
                      <NavLink
                        key={item.path}
                        to={item.path}
                        end={item.path === '/'}
                        className={({ isActive }) =>
                          `flex items-center gap-3 w-full px-3 py-2 rounded-lg text-sm transition-colors ${
                            isActive
                              ? 'bg-blue-600/20 text-blue-400'
                              : 'text-slate-300 hover:bg-slate-700/50 hover:text-slate-100'
                          }`
                        }
                      >
                        <item.icon className="w-4 h-4" />
                        <span>{item.label}</span>
                      </NavLink>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </nav>

          {/* Right side controls */}
          <div className="flex items-center gap-3 ml-auto">
            {/* Search */}
            <button
              onClick={handleToggleSearch}
              className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/50 transition-colors"
            >
              <Search className="w-4 h-4" />
              <span className="hidden sm:inline">Search</span>
              <kbd className="hidden sm:inline px-1.5 py-0.5 rounded bg-slate-700 text-xs text-slate-400">/</kbd>
            </button>

            {/* Theme toggle */}
            <button
              onClick={toggleTheme}
              className="h-8 w-8 rounded-lg hover:bg-slate-800 flex items-center justify-center transition-colors text-slate-400 hover:text-slate-200"
              title={isDark ? 'Light mode' : 'Dark mode'}
              aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>

            {/* User / Logout */}
            <div className="hidden sm:flex items-center gap-2 pl-3 border-l border-slate-700">
              <span className="text-xs text-slate-400 flex items-center gap-1.5">
                <User className="h-3.5 w-3.5" />
                <span className="hidden lg:inline">
                  {username || 'admin'}
                </span>
              </span>
              <button
                onClick={logout}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg hover:bg-red-500/10 hover:text-red-400 text-slate-400 transition-colors text-xs"
                title="Sign out"
              >
                <LogOut className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">Logout</span>
              </button>
            </div>

            {/* Connection status */}
            <div className="relative group" role="status" aria-label={wsConnected ? 'WebSocket connected' : 'WebSocket disconnected'}>
              <span className={`block w-2.5 h-2.5 rounded-full ${
                wsConnected
                  ? 'bg-green-400 shadow-green-400/50 shadow-sm'
                  : 'bg-red-400 shadow-red-400/50 shadow-sm'
              }`} />
              <div className="absolute right-0 top-full mt-1 px-2 py-1 bg-slate-900 text-xs text-white rounded shadow-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
                {wsConnected ? 'Connected' : 'Disconnected'}
              </div>
            </div>

            {/* Mobile menu button */}
            <button
              onClick={() => setMobileOpen(!mobileOpen)}
              className="h-8 w-8 rounded-lg hover:bg-slate-800 flex md:hidden items-center justify-center transition-colors text-slate-400"
              aria-label={mobileOpen ? 'Close menu' : 'Open menu'}
              aria-expanded={mobileOpen}
            >
              {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </button>
          </div>
        </div>
      </header>

      {/* Mobile menu */}
      {mobileOpen && (
        <div className="fixed inset-0 z-40 md:hidden">
          <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setMobileOpen(false)} />
          <div className="absolute top-14 left-0 right-0 bg-slate-900 border-b border-slate-700 shadow-2xl max-h-[80vh] overflow-y-auto z-50">
            {NAV_GROUPS.map((group) => (
              <div key={group.label} className="px-4 py-3">
                <h3 className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-2">{group.label}</h3>
                <div className="space-y-1">
                  {group.items.map((item) => (
                    <NavLink
                      key={item.path}
                      to={item.path}
                      end={item.path === '/'}
                      className={({ isActive }) =>
                        `flex items-center gap-3 w-full px-3 py-2.5 rounded-lg text-sm transition-colors ${
                          isActive
                            ? 'bg-blue-600/20 text-blue-400'
                            : 'text-slate-300 hover:bg-slate-800 hover:text-slate-100'
                        }`
                      }
                    >
                      <item.icon className="w-4 h-4" />
                      <span>{item.label}</span>
                    </NavLink>
                  ))}
                </div>
              </div>
            ))}

            {/* Mobile logout */}
            <div className="px-4 py-3 border-t border-slate-700">
              <button
                onClick={logout}
                className="flex items-center gap-3 w-full px-3 py-2.5 rounded-lg text-sm text-red-400 hover:bg-slate-800 transition-colors"
              >
                <LogOut className="w-4 h-4" />
                <span>Logout</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Main content ──────────────────────────────── */}
      <main className="flex-1 overflow-auto px-6 py-6 page-bg">
        <div className="animate-fade-in">
          <Outlet />
        </div>
      </main>

      {/* ── Modals ──────────────────────────────────── */}
      <GlobalSearch isOpen={searchOpen} onClose={() => setSearchOpen(false)} />

      {helpOpen && (
        <div className="modal-backdrop animate-fade-in" onClick={() => setHelpOpen(false)} role="dialog" aria-modal="true" aria-label="Keyboard shortcuts">
          <div className="flex justify-center pt-[10vh]">
            <div className="modal-card w-full max-w-lg overflow-hidden" onClick={(e) => e.stopPropagation()}>
              <div className="flex items-center justify-between px-4 py-3 border-b border-slate-700/50">
                <h2 className="text-lg font-semibold text-white">Keyboard Shortcuts</h2>
                <button onClick={() => setHelpOpen(false)} className="text-slate-400 hover:text-slate-200" aria-label="Close keyboard shortcuts"><X className="w-5 h-5" /></button>
              </div>
              <div className="max-h-[60vh] overflow-y-auto p-4">
                {['General', 'Navigation', 'Actions'].map((category) => (
                  <div key={category} className="mb-4">
                    <h3 className="text-sm font-medium text-slate-500 mb-2">{category}</h3>
                    <div className="space-y-1">
                      {shortcuts.filter((s) => s.category === category).map((s) => (
                        <div key={s.keys} className="flex items-center justify-between py-1.5">
                          <span className="text-sm text-slate-200">{s.description}</span>
                          <kbd className="px-2 py-0.5 rounded bg-slate-700 text-xs text-slate-400 font-mono">{s.keys}</kbd>
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default MainLayout;
