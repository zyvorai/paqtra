import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Box,
  Typography,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TablePagination,
  Chip,
  TextField,
  MenuItem,
  FormControl,
  InputLabel,
  Select,
  Grid,
  Card,
  CardContent,
  IconButton,
  Tooltip,
  Switch,
  FormControlLabel,
  CircularProgress,
  Alert,
  SelectChangeEvent,
} from '@mui/material';
import {
  Refresh,
  Search,
  ArrowForward,
  Block,
  Visibility,
  Speed,
  CallMade,
  CallReceived,
} from '@mui/icons-material';
import axios from 'axios';
import { format, parseISO } from 'date-fns';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface FlowEndpoint {
  namespace: string;
  pod: string;
  ip: string;
}

interface Flow {
  id: string;
  timestamp: string;
  source: FlowEndpoint;
  destination: FlowEndpoint;
  verdict: string;
  protocol: string;
  port: number;
}

interface FlowsResponse {
  flows: Flow[];
  total: number;
  limit: number;
  offset: number;
}

interface FlowStats {
  total_flows: number;
  forwarded: number;
  dropped: number;
  requests_per_second: number;
  avg_latency_ms: number;
}

type Verdict = 'ALL' | 'FORWARDED' | 'DROPPED' | 'AUDIT';

const VERDICT_COLORS: Record<string, 'success' | 'error' | 'warning' | 'default'> = {
  FORWARDED: 'success',
  DROPPED: 'error',
  AUDIT: 'warning',
};

const VERDICT_ICONS: Record<string, React.ReactElement> = {
  FORWARDED: <ArrowForward fontSize="small" />,
  DROPPED: <Block fontSize="small" />,
  AUDIT: <Visibility fontSize="small" />,
};

const REFRESH_INTERVALS = [
  { label: '5s', value: 5000 },
  { label: '10s', value: 10000 },
  { label: '30s', value: 30000 },
  { label: '1m', value: 60000 },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Flows: React.FC = () => {
  // Filter state
  const [namespace, setNamespace] = useState<string>('');
  const [verdict, setVerdict] = useState<Verdict>('ALL');
  const [searchText, setSearchText] = useState<string>('');

  // Pagination state
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(25);

  // Data state
  const [flows, setFlows] = useState<Flow[]>([]);
  const [total, setTotal] = useState(0);
  const [stats, setStats] = useState<FlowStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Auto-refresh state
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [refreshInterval, setRefreshInterval] = useState(10000);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // -------------------------------------------------------------------------
  // Data fetching
  // -------------------------------------------------------------------------

  const fetchFlows = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params: Record<string, string | number> = {
        limit: rowsPerPage,
        offset: page * rowsPerPage,
      };
      if (namespace) params.namespace = namespace;
      if (verdict !== 'ALL') params.verdict = verdict;

      const response = await axios.get<FlowsResponse>('/api/v1/flows', { params });
      setFlows(response.data.flows);
      setTotal(response.data.total);
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to fetch flows';
      setError(String(message));
    } finally {
      setLoading(false);
    }
  }, [namespace, verdict, page, rowsPerPage]);

  const fetchStats = useCallback(async () => {
    try {
      const response = await axios.get<FlowStats>('/api/v1/flows/stats');
      setStats(response.data);
    } catch {
      // Stats are non-critical; silently ignore
    }
  }, []);

  // Initial + filter-change fetch
  useEffect(() => {
    fetchFlows();
    fetchStats();
  }, [fetchFlows, fetchStats]);

  // Auto-refresh
  useEffect(() => {
    if (autoRefresh) {
      intervalRef.current = setInterval(() => {
        fetchFlows();
        fetchStats();
      }, refreshInterval);
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [autoRefresh, refreshInterval, fetchFlows, fetchStats]);

  // -------------------------------------------------------------------------
  // Client-side search filter (applied on top of server results)
  // -------------------------------------------------------------------------

  const filteredFlows = searchText
    ? flows.filter((f) => {
        const term = searchText.toLowerCase();
        return (
          f.source.pod.toLowerCase().includes(term) ||
          f.source.namespace.toLowerCase().includes(term) ||
          f.destination.pod.toLowerCase().includes(term) ||
          f.destination.namespace.toLowerCase().includes(term) ||
          f.protocol.toLowerCase().includes(term) ||
          String(f.port).includes(term)
        );
      })
    : flows;

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  const formatTimestamp = (ts: string): string => {
    try {
      return format(parseISO(ts), 'yyyy-MM-dd HH:mm:ss');
    } catch {
      return ts;
    }
  };

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        Flow Monitoring
      </Typography>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {/* ---- Stats cards ---- */}
      <Grid container spacing={2} sx={{ mb: 3 }}>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <Speed color="primary" fontSize="small" />
                <Typography variant="body2" color="text.secondary" sx={{ ml: 1 }}>
                  Total Flows
                </Typography>
              </Box>
              <Typography variant="h5">{stats?.total_flows ?? 0}</Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <CallMade color="success" fontSize="small" />
                <Typography variant="body2" color="text.secondary" sx={{ ml: 1 }}>
                  Forwarded
                </Typography>
              </Box>
              <Typography variant="h5" color="success.main">
                {stats?.forwarded ?? 0}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <CallReceived color="error" fontSize="small" />
                <Typography variant="body2" color="text.secondary" sx={{ ml: 1 }}>
                  Dropped
                </Typography>
              </Box>
              <Typography variant="h5" color="error.main">
                {stats?.dropped ?? 0}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card>
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <Speed color="info" fontSize="small" />
                <Typography variant="body2" color="text.secondary" sx={{ ml: 1 }}>
                  Req/s
                </Typography>
              </Box>
              <Typography variant="h5">{stats?.requests_per_second ?? 0}</Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* ---- Filter bar ---- */}
      <Paper sx={{ p: 2, mb: 2 }}>
        <Grid container spacing={2} alignItems="center">
          <Grid item xs={12} sm={3}>
            <TextField
              fullWidth
              size="small"
              label="Namespace"
              value={namespace}
              onChange={(e) => {
                setNamespace(e.target.value);
                setPage(0);
              }}
              placeholder="e.g. default"
            />
          </Grid>
          <Grid item xs={12} sm={2}>
            <FormControl fullWidth size="small">
              <InputLabel>Verdict</InputLabel>
              <Select
                value={verdict}
                label="Verdict"
                onChange={(e: SelectChangeEvent) => {
                  setVerdict(e.target.value as Verdict);
                  setPage(0);
                }}
              >
                <MenuItem value="ALL">All</MenuItem>
                <MenuItem value="FORWARDED">Forwarded</MenuItem>
                <MenuItem value="DROPPED">Dropped</MenuItem>
                <MenuItem value="AUDIT">Audit</MenuItem>
              </Select>
            </FormControl>
          </Grid>
          <Grid item xs={12} sm={3}>
            <TextField
              fullWidth
              size="small"
              label="Search"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
              InputProps={{
                startAdornment: <Search fontSize="small" sx={{ mr: 0.5, color: 'text.secondary' }} />,
              }}
              placeholder="pod, protocol, port..."
            />
          </Grid>
          <Grid item xs={12} sm={4}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <FormControlLabel
                control={
                  <Switch
                    checked={autoRefresh}
                    onChange={(e) => setAutoRefresh(e.target.checked)}
                    size="small"
                  />
                }
                label="Auto-refresh"
              />
              {autoRefresh && (
                <FormControl size="small" sx={{ minWidth: 80 }}>
                  <Select
                    value={refreshInterval}
                    onChange={(e) => setRefreshInterval(Number(e.target.value))}
                    size="small"
                  >
                    {REFRESH_INTERVALS.map((ri) => (
                      <MenuItem key={ri.value} value={ri.value}>
                        {ri.label}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
              )}
              <Tooltip title="Refresh now">
                <IconButton
                  onClick={() => {
                    fetchFlows();
                    fetchStats();
                  }}
                  size="small"
                >
                  <Refresh />
                </IconButton>
              </Tooltip>
            </Box>
          </Grid>
        </Grid>
      </Paper>

      {/* ---- Flow table ---- */}
      <Paper sx={{ width: '100%', overflow: 'hidden' }}>
        {loading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
            <CircularProgress size={28} />
          </Box>
        )}

        <TableContainer sx={{ maxHeight: 600 }}>
          <Table stickyHeader size="small">
            <TableHead>
              <TableRow>
                <TableCell>Timestamp</TableCell>
                <TableCell>Source</TableCell>
                <TableCell>Destination</TableCell>
                <TableCell>Protocol</TableCell>
                <TableCell align="right">Port</TableCell>
                <TableCell>Verdict</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filteredFlows.length === 0 && !loading ? (
                <TableRow>
                  <TableCell colSpan={6} align="center">
                    <Typography variant="body2" color="text.secondary" sx={{ py: 4 }}>
                      {error ? 'Unable to load flows.' : 'No flows found. Connect to Hubble to see network flows.'}
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                filteredFlows.map((flow) => (
                  <TableRow key={flow.id} hover>
                    <TableCell sx={{ whiteSpace: 'nowrap' }}>
                      <Typography variant="body2">{formatTimestamp(flow.timestamp)}</Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        {flow.source.namespace}/{flow.source.pod}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {flow.source.ip}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        {flow.destination.namespace}/{flow.destination.pod}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {flow.destination.ip}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Chip label={flow.protocol} size="small" variant="outlined" />
                    </TableCell>
                    <TableCell align="right">
                      <Typography variant="body2">{flow.port}</Typography>
                    </TableCell>
                    <TableCell>
                      <Chip
                        icon={VERDICT_ICONS[flow.verdict]}
                        label={flow.verdict}
                        size="small"
                        color={VERDICT_COLORS[flow.verdict] ?? 'default'}
                      />
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>

        <TablePagination
          component="div"
          count={total}
          page={page}
          onPageChange={(_, newPage) => setPage(newPage)}
          rowsPerPage={rowsPerPage}
          onRowsPerPageChange={(e) => {
            setRowsPerPage(parseInt(e.target.value, 10));
            setPage(0);
          }}
          rowsPerPageOptions={[10, 25, 50, 100]}
        />
      </Paper>
    </Box>
  );
};

export default Flows;
