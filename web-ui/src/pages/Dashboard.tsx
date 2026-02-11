import React, { useEffect, useState } from 'react';
import {
  Box,
  Grid,
  Paper,
  Typography,
  Card,
  CardContent,
} from '@mui/material';
import {
  Timeline,
  Speed,
  Error,
  CheckCircle,
} from '@mui/icons-material';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

interface MetricData {
  timestamp: string;
  requests_per_sec: number;
  avg_latency_ms: number;
  error_rate: number;
}

const Dashboard: React.FC = () => {
  const [metrics, setMetrics] = useState<MetricData[]>([]);

  useEffect(() => {
    // TODO: Connect to WebSocket for real-time metrics
    const ws = new WebSocket('ws://localhost:8080/api/v1/ws/metrics');

    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setMetrics((prev) => [...prev.slice(-20), data]);
    };

    return () => ws.close();
  }, []);

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        Dashboard
      </Typography>

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
                {metrics[metrics.length - 1]?.requests_per_sec.toFixed(1) || '0.0'}
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
                {metrics[metrics.length - 1]?.avg_latency_ms.toFixed(1) || '0.0'} ms
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <Error color="error" />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Error Rate
                </Typography>
              </Box>
              <Typography variant="h4">
                {((metrics[metrics.length - 1]?.error_rate || 0) * 100).toFixed(2)}%
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={3}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                <CheckCircle color="success" />
                <Typography variant="h6" sx={{ ml: 1 }}>
                  Status
                </Typography>
              </Box>
              <Typography variant="h4" color="success.main">
                Healthy
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
