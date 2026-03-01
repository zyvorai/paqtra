import React, { useEffect, useState, useRef, useCallback } from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
  Alert,
} from '@mui/material';
import {
  Timeline,
  Speed,
  Error as ErrorIcon,
  CheckCircle,
} from '@mui/icons-material';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

interface MetricData {
  timestamp: string;
  requests_per_sec: number;
  avg_latency_ms: number;
  error_rate: number;
}

/** Build WebSocket URL relative to the current page location */
function getWsUrl(path: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = window.location.host;
  return `${protocol}//${host}${path}`;
}

const MAX_METRICS = 20;
const RECONNECT_DELAY_MS = 3000;

const Dashboard: React.FC = () => {
  const [metrics, setMetrics] = useState<MetricData[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    setConnectionStatus('connecting');
    const ws = new WebSocket(getWsUrl('/api/v1/ws/metrics'));
    wsRef.current = ws;

    ws.onopen = () => {
      setConnectionStatus('connected');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.timestamp) {
          setMetrics((prev) => [...prev.slice(-(MAX_METRICS - 1)), data]);
        }
      } catch {
        // Ignore non-JSON messages (e.g., connection ack)
      }
    };

    ws.onerror = () => {
      setConnectionStatus('disconnected');
    };

    ws.onclose = () => {
      setConnectionStatus('disconnected');
      wsRef.current = null;
      // Auto-reconnect
      reconnectTimerRef.current = setTimeout(connect, RECONNECT_DELAY_MS);
    };
  }, []);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  const latestMetric = metrics[metrics.length - 1];

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        Dashboard
      </Typography>

      {connectionStatus === 'disconnected' && (
        <Alert severity="warning" sx={{ mb: 2 }}>
          Disconnected from metrics stream. Reconnecting...
        </Alert>
      )}

      <Grid container spacing={3}>
        {/* Key Metrics */}
        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <Speed color="primary" />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Requests/sec
                </Typography>
              </Box>
              <Typography variant="h4">
                {latestMetric?.requests_per_sec.toFixed(1) ?? '0.0'}
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <Timeline color="info" />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Avg Latency
                </Typography>
              </Box>
              <Typography variant="h4">
                {latestMetric?.avg_latency_ms.toFixed(1) ?? '0.0'} ms
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <ErrorIcon color="error" />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Error Rate
                </Typography>
              </Box>
              <Typography variant="h4">
                {((latestMetric?.error_rate ?? 0) * 100).toFixed(2)}%
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <CheckCircle color={connectionStatus === 'connected' ? 'success' : 'disabled'} />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Status
                </Typography>
              </Box>
              <Typography variant="h4" color={connectionStatus === 'connected' ? 'success.main' : 'text.secondary'}>
                {connectionStatus === 'connected' ? 'Healthy' : 'Connecting...'}
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        {/* Request Rate Chart */}
        <Grid item xs={12}>
          <Paper sx={{ p: 2 }}>
            <Typography variant="h6" gutterBottom>
              Request Rate
            </Typography>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="timestamp" />
                <YAxis />
                <Tooltip />
                <Line
                  type="monotone"
                  dataKey="requests_per_sec"
                  stroke="#8884d8"
                  name="Requests/sec"
                />
              </LineChart>
            </ResponsiveContainer>
          </Paper>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Dashboard;
