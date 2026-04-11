import React, { useState, useEffect, useRef } from 'react';
import {
  Settings as SettingsIcon,
  Save,
  RotateCcw,
  Wifi,
  Loader2,
  CheckCircle,
  XCircle,
  Server,
  Palette,
} from 'lucide-react';
import { checkHealth } from '../../services/api';
import { usePageTitle } from '../../hooks/usePageTitle';
import ToggleSwitch from '../../components/ToggleSwitch';

interface AppSettings {
  apiUrl: string;
  hubbleAddress: string;
  refreshInterval: number;
  darkMode: boolean;
  autoRefresh: boolean;
  compactMode: boolean;
}

const STORAGE_KEY = 'cilium-vision-settings';
const DEFAULTS: AppSettings = {
  apiUrl: '/api/v1',
  hubbleAddress: 'hubble-relay.kube-system.svc.cluster.local:4245',
  refreshInterval: 10,
  darkMode: true,
  autoRefresh: true,
  compactMode: false,
};

function loadSettings(): AppSettings {
  try { const s = localStorage.getItem(STORAGE_KEY); return s ? { ...DEFAULTS, ...JSON.parse(s) } : { ...DEFAULTS }; }
  catch { return { ...DEFAULTS }; }
}

const Settings: React.FC = () => {
  usePageTitle('Settings');
  const [settings, setSettings] = useState<AppSettings>(loadSettings);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [apiStatus, setApiStatus] = useState<'connected' | 'disconnected' | 'checking'>('checking');
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    checkApi();
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, []);

  const checkApi = async () => {
    setApiStatus('checking');
    try { await checkHealth(); setApiStatus('connected'); }
    catch { setApiStatus('disconnected'); }
  };

  const handleSave = () => {
    // Validate API base URL before saving
    const url = settings.apiUrl.trim();
    if (url.startsWith('/')) {
      // Relative paths are allowed
    } else if (url.startsWith('http://') || url.startsWith('https://')) {
      try {
        const parsed = new URL(url);
        if (parsed.origin !== window.location.origin) {
          setError('API URL must be same-origin. Cross-origin URLs are not allowed.');
          return;
        }
      } catch {
        setError('Invalid API URL format.');
        return;
      }
    } else {
      setError('API URL must start with /, http://, or https://.');
      return;
    }
    // Check for suspicious characters (e.g. javascript:, data:, whitespace, angle brackets)
    if (/[<>"'`\s]|javascript:|data:/i.test(url)) {
      setError('API URL contains invalid characters.');
      return;
    }

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
      setSaved(true); setError(null);
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => setSaved(false), 3000);
    } catch { setError('Failed to save settings'); }
  };

  const handleReset = () => { setSettings({ ...DEFAULTS }); setSaved(false); };

  const update = (key: keyof AppSettings, value: AppSettings[keyof AppSettings]) => {
    setSettings((s) => ({ ...s, [key]: value }));
    setSaved(false);
  };

  return (
    <div>
      <div className="mb-6">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-slate-500 to-slate-700 flex items-center justify-center shadow-lg shadow-slate-500/20">
            <SettingsIcon className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white">Settings</h1>
            <p className="text-sm text-slate-400">Application configuration</p>
          </div>
        </div>
      </div>

      {saved && <div className="mb-4 p-3 rounded-xl bg-green-500/10 border border-green-500/30 text-green-400 text-sm">Settings saved to local storage.</div>}
      {error && <div className="mb-4 p-3 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-sm">{error}</div>}

      <div className="space-y-6">
        {/* Connection Status */}
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2 mb-4">
            <Wifi className="w-5 h-5 text-green-400" />
            Connection Status
          </h2>
          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 rounded-lg bg-slate-900/50">
              <div className="flex items-center gap-3">
                {apiStatus === 'connected' ? <CheckCircle className="w-5 h-5 text-green-400" /> : apiStatus === 'checking' ? <Loader2 className="w-5 h-5 animate-spin text-slate-400" /> : <XCircle className="w-5 h-5 text-red-400" />}
                <div>
                  <div className="text-sm font-medium text-white">API Server</div>
                  <div className="text-xs text-slate-400">/health endpoint</div>
                </div>
              </div>
              <button onClick={checkApi} className="px-3 py-1.5 rounded-lg border border-slate-700/50 text-xs text-slate-400 hover:text-white hover:bg-slate-700/30 transition-colors">
                Test
              </button>
            </div>
            <div className="flex items-center justify-between p-3 rounded-lg bg-slate-900/50">
              <div className="flex items-center gap-3">
                <XCircle className="w-5 h-5 text-slate-500" />
                <div>
                  <div className="text-sm font-medium text-white">Hubble Relay</div>
                  <div className="text-xs text-slate-400">gRPC connection</div>
                </div>
              </div>
              <span className="text-xs text-slate-500">Not connected</span>
            </div>
          </div>
        </div>

        {/* Server Configuration */}
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2 mb-4">
            <Server className="w-5 h-5 text-purple-400" />
            Server Configuration
          </h2>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-slate-300 mb-1.5">API Base URL</label>
              <input value={settings.apiUrl} onChange={(e) => update('apiUrl', e.target.value)}
                className="w-full px-3 py-2.5 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500" />
              <p className="text-xs text-slate-500 mt-1">e.g., /api/v1 or http://localhost:9191/api/v1</p>
            </div>
            <div>
              <label className="block text-sm text-slate-300 mb-1.5">Hubble Relay Address</label>
              <input value={settings.hubbleAddress} onChange={(e) => update('hubbleAddress', e.target.value)}
                className="w-full px-3 py-2.5 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500" />
            </div>
            <div>
              <label className="block text-sm text-slate-300 mb-1.5">Auto-refresh Interval (seconds)</label>
              <input type="number" min={1} max={300} value={settings.refreshInterval} onChange={(e) => update('refreshInterval', Math.max(1, parseInt(e.target.value) || 1))}
                className="w-full px-3 py-2.5 rounded-lg bg-slate-900 border border-slate-700 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
            </div>
          </div>
        </div>

        {/* Display Preferences */}
        <div className="rounded-xl border border-slate-700/50 bg-slate-800/50 p-6">
          <h2 className="text-lg font-semibold text-white flex items-center gap-2 mb-4">
            <Palette className="w-5 h-5 text-pink-400" />
            Display Preferences
          </h2>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-slate-300">Dark Mode</div>
                <div className="text-xs text-slate-500">Use dark color scheme</div>
              </div>
              <ToggleSwitch checked={settings.darkMode} onChange={(v) => update('darkMode', v)} />
            </div>
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-slate-300">Auto-refresh</div>
                <div className="text-xs text-slate-500">Automatically refresh data at set interval</div>
              </div>
              <ToggleSwitch checked={settings.autoRefresh} onChange={(v) => update('autoRefresh', v)} />
            </div>
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-slate-300">Compact Mode</div>
                <div className="text-xs text-slate-500">Reduce spacing and padding in views</div>
              </div>
              <ToggleSwitch checked={settings.compactMode} onChange={(v) => update('compactMode', v)} />
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-3 pt-2">
          <button onClick={handleReset} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-slate-700 to-slate-800 text-slate-300 hover:text-white text-sm shadow-sm transition-colors">
            <RotateCcw className="w-4 h-4" /> Reset Defaults
          </button>
          <button onClick={handleSave} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-600 to-blue-700 text-white text-sm hover:from-blue-500 hover:to-blue-600 transition-all">
            <Save className="w-4 h-4" /> Save Settings
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
