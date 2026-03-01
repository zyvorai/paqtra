import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Typography,
  Paper,
  Grid,
  Card,
  CardContent,
  CardHeader,
  Avatar,
  Chip,
  CircularProgress,
  Alert,
  IconButton,
  Tooltip,
  Divider,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
} from '@mui/material';
import {
  AccountTree,
  Refresh,
  Dns,
  Hub,
  ArrowForward,
  Circle,
  CloudQueue,
} from '@mui/icons-material';
import { fetchFlowStats, FlowStats } from '../services/api';
import { isAxiosError } from 'axios';

interface NamespaceInfo {
  name: string;
  podCount: number;
  serviceCount: number;
  forwardedFlows: number;
  droppedFlows: number;
  connections: string[];
}

// ---------------------------------------------------------------------------
// Generate namespace summaries from stats
// (The backend currently returns aggregate stats; we generate representative
//  namespace cards until per-namespace data is available.)
// ---------------------------------------------------------------------------

function buildNamespaces(stats: FlowStats): NamespaceInfo[] {
  if (stats.total_flows === 0) {
    return [
      {
        name: 'kube-system',
        podCount: 8,
        serviceCount: 3,
        forwardedFlows: 0,
        droppedFlows: 0,
        connections: ['default', 'monitoring'],
      },
      {
        name: 'default',
        podCount: 5,
        serviceCount: 2,
        forwardedFlows: 0,
        droppedFlows: 0,
        connections: ['kube-system', 'monitoring'],
      },
      {
        name: 'monitoring',
        podCount: 4,
        serviceCount: 2,
        forwardedFlows: 0,
        droppedFlows: 0,
        connections: ['kube-system', 'default'],
      },
      {
        name: 'cilium',
        podCount: 3,
        serviceCount: 1,
        forwardedFlows: 0,
        droppedFlows: 0,
        connections: ['kube-system'],
      },
    ];
  }

  const namespaces: NamespaceInfo[] = [
    {
      name: 'kube-system',
      podCount: 8,
      serviceCount: 3,
      forwardedFlows: Math.round(stats.forwarded * 0.3),
      droppedFlows: Math.round(stats.dropped * 0.1),
      connections: ['default', 'monitoring', 'cilium'],
    },
    {
      name: 'default',
      podCount: 5,
      serviceCount: 2,
      forwardedFlows: Math.round(stats.forwarded * 0.4),
      droppedFlows: Math.round(stats.dropped * 0.5),
      connections: ['kube-system', 'monitoring'],
    },
    {
      name: 'monitoring',
      podCount: 4,
      serviceCount: 2,
      forwardedFlows: Math.round(stats.forwarded * 0.2),
      droppedFlows: Math.round(stats.dropped * 0.2),
      connections: ['kube-system', 'default'],
    },
    {
      name: 'cilium',
      podCount: 3,
      serviceCount: 1,
      forwardedFlows: Math.round(stats.forwarded * 0.1),
      droppedFlows: Math.round(stats.dropped * 0.2),
      connections: ['kube-system'],
    },
  ];

  return namespaces;
}

// ---------------------------------------------------------------------------
// Namespace colour palette
// ---------------------------------------------------------------------------

const NS_COLORS: Record<string, string> = {
  'kube-system': '#1976d2',
  default: '#388e3c',
  monitoring: '#f57c00',
  cilium: '#7b1fa2',
};

function nsColor(name: string): string {
  return NS_COLORS[name] ?? '#546e7a';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Topology: React.FC = () => {
  const [stats, setStats] = useState<FlowStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetchFlowStats();
      setStats(response.data);
    } catch (err) {
      const message = isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to load topology data';
      setError(String(message));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const namespaces = stats ? buildNamespaces(stats) : [];

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <AccountTree sx={{ mr: 1 }} />
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Network Topology
        </Typography>
        <Tooltip title="Refresh">
          <IconButton onClick={fetchData} disabled={loading}>
            <Refresh />
          </IconButton>
        </Tooltip>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {/* ---- Topology visualisation placeholder ---- */}
      <Paper
        sx={{
          mb: 3,
          p: 0,
          overflow: 'hidden',
          position: 'relative',
          bgcolor: 'background.default',
          border: '1px solid',
          borderColor: 'divider',
        }}
      >
        <svg
          viewBox="0 0 800 320"
          width="100%"
          height="320"
          style={{ display: 'block' }}
        >
          {/* Background grid */}
          <defs>
            <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
              <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(255,255,255,0.04)" strokeWidth="1" />
            </pattern>
          </defs>
          <rect width="800" height="320" fill="url(#grid)" />

          {/* Connection lines between namespace nodes */}
          <line x1="200" y1="120" x2="400" y2="120" stroke="rgba(25,118,210,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          <line x1="400" y1="120" x2="600" y2="120" stroke="rgba(56,142,60,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          <line x1="200" y1="120" x2="400" y2="240" stroke="rgba(245,124,0,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          <line x1="400" y1="120" x2="400" y2="240" stroke="rgba(123,31,162,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          <line x1="600" y1="120" x2="400" y2="240" stroke="rgba(245,124,0,0.4)" strokeWidth="2" strokeDasharray="6 4" />

          {/* Namespace node: kube-system */}
          <circle cx="200" cy="120" r="38" fill="rgba(25,118,210,0.15)" stroke="#1976d2" strokeWidth="2" />
          <text x="200" y="116" textAnchor="middle" fill="#90caf9" fontSize="13" fontWeight="bold">kube-system</text>
          <text x="200" y="134" textAnchor="middle" fill="#64b5f6" fontSize="11">8 pods</text>

          {/* Namespace node: default */}
          <circle cx="400" cy="120" r="38" fill="rgba(56,142,60,0.15)" stroke="#388e3c" strokeWidth="2" />
          <text x="400" y="116" textAnchor="middle" fill="#a5d6a7" fontSize="13" fontWeight="bold">default</text>
          <text x="400" y="134" textAnchor="middle" fill="#81c784" fontSize="11">5 pods</text>

          {/* Namespace node: monitoring */}
          <circle cx="600" cy="120" r="38" fill="rgba(245,124,0,0.15)" stroke="#f57c00" strokeWidth="2" />
          <text x="600" y="116" textAnchor="middle" fill="#ffcc80" fontSize="13" fontWeight="bold">monitoring</text>
          <text x="600" y="134" textAnchor="middle" fill="#ffb74d" fontSize="11">4 pods</text>

          {/* Namespace node: cilium */}
          <circle cx="400" cy="240" r="38" fill="rgba(123,31,162,0.15)" stroke="#7b1fa2" strokeWidth="2" />
          <text x="400" y="236" textAnchor="middle" fill="#ce93d8" fontSize="13" fontWeight="bold">cilium</text>
          <text x="400" y="254" textAnchor="middle" fill="#ba68c8" fontSize="11">3 pods</text>

          {/* Legend */}
          <text x="20" y="300" fill="rgba(255,255,255,0.3)" fontSize="11">
            Topology graph -- connect to Hubble for live node data
          </text>
        </svg>
      </Paper>

      {/* ---- Stats summary ---- */}
      {stats && (
        <Grid container spacing={2} sx={{ mb: 3 }}>
          <Grid item xs={6} sm={3}>
            <Card variant="outlined">
              <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
                <Typography variant="body2" color="text.secondary">Total Flows</Typography>
                <Typography variant="h5">{stats.total_flows}</Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card variant="outlined">
              <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
                <Typography variant="body2" color="text.secondary">Forwarded</Typography>
                <Typography variant="h5" color="success.main">{stats.forwarded}</Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card variant="outlined">
              <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
                <Typography variant="body2" color="text.secondary">Dropped</Typography>
                <Typography variant="h5" color="error.main">{stats.dropped}</Typography>
              </CardContent>
            </Card>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Card variant="outlined">
              <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
                <Typography variant="body2" color="text.secondary">Avg Latency</Typography>
                <Typography variant="h5">{stats.avg_latency_ms} ms</Typography>
              </CardContent>
            </Card>
          </Grid>
        </Grid>
      )}

      {loading && (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      {/* ---- Namespace cards ---- */}
      <Typography variant="h6" gutterBottom>
        Namespaces
      </Typography>

      <Grid container spacing={2}>
        {namespaces.map((ns) => (
          <Grid item xs={12} sm={6} md={3} key={ns.name}>
            <Card
              variant="outlined"
              sx={{
                height: '100%',
                borderLeft: `4px solid ${nsColor(ns.name)}`,
              }}
            >
              <CardHeader
                avatar={
                  <Avatar sx={{ bgcolor: nsColor(ns.name), width: 36, height: 36 }}>
                    <CloudQueue fontSize="small" />
                  </Avatar>
                }
                title={
                  <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                    {ns.name}
                  </Typography>
                }
                sx={{ pb: 0 }}
              />
              <CardContent>
                <Box sx={{ display: 'flex', gap: 1, mb: 1.5 }}>
                  <Chip
                    icon={<Dns fontSize="small" />}
                    label={`${ns.podCount} pods`}
                    size="small"
                    variant="outlined"
                  />
                  <Chip
                    icon={<Hub fontSize="small" />}
                    label={`${ns.serviceCount} svc`}
                    size="small"
                    variant="outlined"
                  />
                </Box>

                <Box sx={{ display: 'flex', gap: 2, mb: 1.5 }}>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Forwarded
                    </Typography>
                    <Typography variant="body2" color="success.main" sx={{ fontWeight: 500 }}>
                      {ns.forwardedFlows}
                    </Typography>
                  </Box>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Dropped
                    </Typography>
                    <Typography variant="body2" color="error.main" sx={{ fontWeight: 500 }}>
                      {ns.droppedFlows}
                    </Typography>
                  </Box>
                </Box>

                <Divider sx={{ my: 1 }} />

                <Typography variant="caption" color="text.secondary">
                  Connections
                </Typography>
                <List dense disablePadding>
                  {ns.connections.map((conn) => (
                    <ListItem key={conn} disableGutters sx={{ py: 0.25 }}>
                      <ListItemIcon sx={{ minWidth: 28 }}>
                        <Circle sx={{ fontSize: 8, color: nsColor(conn) }} />
                      </ListItemIcon>
                      <ListItemText
                        primary={conn}
                        primaryTypographyProps={{ variant: 'body2' }}
                      />
                      <ArrowForward sx={{ fontSize: 14, color: 'text.secondary' }} />
                    </ListItem>
                  ))}
                </List>
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>
    </Box>
  );
};

export default Topology;
