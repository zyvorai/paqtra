import React, { useState, useEffect } from 'react';
import {
  Box,
  Typography,
  Paper,
  TextField,
  Button,
  Grid,
  Divider,
  Switch,
  FormControlLabel,
  Alert,
  Card,
  CardContent,
  Chip,
  IconButton,
  Tooltip,
} from '@mui/material';
import {
  Save,
  RestartAlt,
  Settings as SettingsIcon,
  Wifi,
  WifiOff,
  CheckCircle,
  Cancel,
  Circle,
} from '@mui/icons-material';
import { checkHealth } from '../services/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AppSettings {
  apiUrl: string;
  hubbleAddress: string;
  refreshInterval: number;
  darkMode: boolean;
}

interface ConnectionStatus {
  api: 'connected' | 'disconnected' | 'checking';
  hubble: 'connected' | 'disconnected' | 'checking';
}

const STORAGE_KEY = 'cilium-vision-settings';

const DEFAULT_SETTINGS: AppSettings = {
  apiUrl: '/api/v1',
  hubbleAddress: 'hubble-relay.kube-system.svc.cluster.local:4245',
  refreshInterval: 10,
  darkMode: true,
};

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

function loadSettings(): AppSettings {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
    }
  } catch {
    // Ignore corrupt data
  }
  return { ...DEFAULT_SETTINGS };
}

function saveSettings(settings: AppSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Settings: React.FC = () => {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({
    api: 'checking',
    hubble: 'disconnected',
  });

  // -------------------------------------------------------------------------
  // Check API connectivity on mount
  // -------------------------------------------------------------------------

  useEffect(() => {
    checkApiConnection();
  }, []);

  const checkApiConnection = async () => {
    setConnectionStatus((prev) => ({ ...prev, api: 'checking' }));
    try {
      await checkHealth();
      setConnectionStatus((prev) => ({ ...prev, api: 'connected' }));
    } catch {
      setConnectionStatus((prev) => ({ ...prev, api: 'disconnected' }));
    }
  };

  // -------------------------------------------------------------------------
  // Handlers
  // -------------------------------------------------------------------------

  const handleChange = (field: keyof AppSettings) => (
    e: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value =
      field === 'refreshInterval'
        ? Math.max(1, parseInt(e.target.value, 10) || 1)
        : e.target.value;
    setSettings((prev) => ({ ...prev, [field]: value }));
    setSaved(false);
  };

  const handleToggleDarkMode = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSettings((prev) => ({ ...prev, darkMode: e.target.checked }));
    setSaved(false);
  };

  const handleSave = () => {
    try {
      saveSettings(settings);
      setSaved(true);
      setError(null);
      // Auto-dismiss success message
      setTimeout(() => setSaved(false), 3000);
    } catch (err) {
      setError('Failed to save settings');
    }
  };

  const handleReset = () => {
    setSettings({ ...DEFAULT_SETTINGS });
    setSaved(false);
  };

  // -------------------------------------------------------------------------
  // Connection status rendering
  // -------------------------------------------------------------------------

  const statusChip = (status: 'connected' | 'disconnected' | 'checking', label: string) => {
    switch (status) {
      case 'connected':
        return (
          <Chip
            icon={<CheckCircle fontSize="small" />}
            label={`${label}: Connected`}
            color="success"
            size="small"
            variant="outlined"
          />
        );
      case 'disconnected':
        return (
          <Chip
            icon={<Cancel fontSize="small" />}
            label={`${label}: Disconnected`}
            color="error"
            size="small"
            variant="outlined"
          />
        );
      case 'checking':
        return (
          <Chip
            icon={<Circle fontSize="small" />}
            label={`${label}: Checking...`}
            size="small"
            variant="outlined"
          />
        );
    }
  };

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <SettingsIcon sx={{ mr: 1 }} />
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Settings
        </Typography>
      </Box>

      {saved && (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setSaved(false)}>
          Settings saved to local storage.
        </Alert>
      )}
      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      <Grid container spacing={3}>
        {/* ---- Connection Status ---- */}
        <Grid item xs={12} md={4}>
          <Card variant="outlined" sx={{ height: '100%' }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Connection Status
              </Typography>
              <Divider sx={{ mb: 2 }} />

              <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  {statusChip(connectionStatus.api, 'API Server')}
                  <Tooltip title="Re-check connection">
                    <IconButton size="small" onClick={checkApiConnection}>
                      {connectionStatus.api === 'connected' ? (
                        <Wifi color="success" fontSize="small" />
                      ) : (
                        <WifiOff color="error" fontSize="small" />
                      )}
                    </IconButton>
                  </Tooltip>
                </Box>

                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  {statusChip(connectionStatus.hubble, 'Hubble Relay')}
                  <Tooltip title="Re-check connection">
                    <IconButton size="small" disabled>
                      <WifiOff color="disabled" fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </Box>
              </Box>

              <Divider sx={{ my: 2 }} />

              <Typography variant="caption" color="text.secondary">
                The API server connection is verified by pinging the /health endpoint.
                Hubble Relay connectivity depends on your Kubernetes cluster configuration.
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        {/* ---- Settings form ---- */}
        <Grid item xs={12} md={8}>
          <Paper sx={{ p: 3 }}>
            <Typography variant="h6" gutterBottom>
              Application Configuration
            </Typography>
            <Divider sx={{ mb: 3 }} />

            <Grid container spacing={3}>
              {/* API URL */}
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="API Base URL"
                  value={settings.apiUrl}
                  onChange={handleChange('apiUrl')}
                  helperText="Base URL for the Cilium Vision API server (e.g., /api/v1 or http://localhost:9191/api/v1)"
                  InputProps={{
                    sx: { fontFamily: 'monospace' },
                  }}
                />
              </Grid>

              {/* Hubble Address */}
              <Grid item xs={12}>
                <TextField
                  fullWidth
                  label="Hubble Relay Address"
                  value={settings.hubbleAddress}
                  onChange={handleChange('hubbleAddress')}
                  helperText="gRPC address for Hubble Relay (e.g., hubble-relay.kube-system.svc.cluster.local:4245)"
                  InputProps={{
                    sx: { fontFamily: 'monospace' },
                  }}
                />
              </Grid>

              {/* Refresh Interval */}
              <Grid item xs={12} sm={6}>
                <TextField
                  fullWidth
                  label="Refresh Interval (seconds)"
                  type="number"
                  value={settings.refreshInterval}
                  onChange={handleChange('refreshInterval')}
                  helperText="How often dashboards auto-refresh data"
                  inputProps={{ min: 1, max: 300 }}
                />
              </Grid>

              {/* Theme Toggle */}
              <Grid item xs={12} sm={6}>
                <Box sx={{ pt: 1 }}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={settings.darkMode}
                        onChange={handleToggleDarkMode}
                      />
                    }
                    label="Dark Mode"
                  />
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
                    Toggle between dark and light theme. Requires app reload for full effect.
                  </Typography>
                </Box>
              </Grid>
            </Grid>

            <Divider sx={{ my: 3 }} />

            {/* Action buttons */}
            <Box sx={{ display: 'flex', gap: 2, justifyContent: 'flex-end' }}>
              <Button
                variant="outlined"
                startIcon={<RestartAlt />}
                onClick={handleReset}
              >
                Reset to Defaults
              </Button>
              <Button
                variant="contained"
                startIcon={<Save />}
                onClick={handleSave}
              >
                Save Settings
              </Button>
            </Box>
          </Paper>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Settings;
