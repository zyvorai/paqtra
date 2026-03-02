import React, { useState, useEffect, useCallback } from 'react';
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
  Grid,
  Card,
  CardContent,
  Chip,
  Button,
  IconButton,
  Tooltip,
  CircularProgress,
  Alert,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
} from '@mui/material';
import {
  Warning,
  Error as ErrorIcon,
  Info,
  Refresh,
  Build,
  CheckCircle,
  ReportProblem,
  BugReport,
} from '@mui/icons-material';
import { fetchAnomalies as apiFetchAnomalies, remediateAnomaly as apiRemediateAnomaly, Anomaly } from '../services/api';
import { isAxiosError } from 'axios';
import { format, parseISO } from 'date-fns';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info';

const SEVERITY_CONFIG: Record<Severity, { color: 'error' | 'warning' | 'info' | 'success' | 'default'; icon: React.ReactElement }> = {
  critical: { color: 'error', icon: <ErrorIcon fontSize="small" /> },
  high: { color: 'warning', icon: <ReportProblem fontSize="small" /> },
  medium: { color: 'info', icon: <Warning fontSize="small" /> },
  low: { color: 'default', icon: <Info fontSize="small" /> },
  info: { color: 'success', icon: <Info fontSize="small" /> },
};

const STATUS_COLORS: Record<string, 'success' | 'warning' | 'error' | 'info' | 'default'> = {
  open: 'error',
  investigating: 'warning',
  remediated: 'success',
  resolved: 'success',
  dismissed: 'default',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Anomalies: React.FC = () => {
  const [anomalies, setAnomalies] = useState<Anomaly[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Remediate confirmation
  const [remediateOpen, setRemediateOpen] = useState(false);
  const [remediatingId, setRemediatingId] = useState<string | null>(null);
  const [remediating, setRemediating] = useState(false);

  // -------------------------------------------------------------------------
  // Data fetching
  // -------------------------------------------------------------------------

  const fetchAnomalies = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await apiFetchAnomalies();
      setAnomalies(response.data.anomalies ?? []);
    } catch (err) {
      const message = isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to fetch anomalies';
      setError(String(message));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAnomalies();
  }, [fetchAnomalies]);

  // -------------------------------------------------------------------------
  // Remediation
  // -------------------------------------------------------------------------

  const handleRemediate = async () => {
    if (!remediatingId) return;
    setRemediating(true);
    setError(null);
    try {
      await apiRemediateAnomaly(remediatingId);
      setRemediateOpen(false);
      setRemediatingId(null);
      setSuccessMessage('Remediation initiated successfully');
      fetchAnomalies();
    } catch (err) {
      const message = isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Remediation failed';
      setError(String(message));
    } finally {
      setRemediating(false);
    }
  };

  // -------------------------------------------------------------------------
  // Stats
  // -------------------------------------------------------------------------

  const countBySeverity = (s: Severity) => anomalies.filter((a) => a.severity === s).length;
  const totalOpen = anomalies.filter((a) => a.status !== 'remediated' && a.status !== 'resolved' && a.status !== 'dismissed').length;

  const formatTimestamp = (ts: string): string => {
    try {
      return format(parseISO(ts), 'yyyy-MM-dd HH:mm:ss');
    } catch {
      return ts || '-';
    }
  };

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <BugReport sx={{ mr: 1 }} />
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Anomaly Detection
        </Typography>
        <Tooltip title="Refresh">
          <IconButton onClick={fetchAnomalies} disabled={loading}>
            <Refresh />
          </IconButton>
        </Tooltip>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}
      {successMessage && (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setSuccessMessage(null)}>
          {successMessage}
        </Alert>
      )}

      {/* ---- Summary cards ---- */}
      <Grid container spacing={2} sx={{ mb: 3 }}>
        <Grid item xs={6} sm={3}>
          <Card
            variant="outlined"
            sx={{ borderLeft: '4px solid', borderLeftColor: 'text.secondary' }}
          >
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <Warning fontSize="small" sx={{ mr: 1, color: 'text.secondary' }} />
                <Typography variant="body2" color="text.secondary">Total Anomalies</Typography>
              </Box>
              <Typography variant="h4">{anomalies.length}</Typography>
              <Typography variant="caption" color="text.secondary">
                {totalOpen} open
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card
            variant="outlined"
            sx={{ borderLeft: '4px solid', borderLeftColor: 'error.main' }}
          >
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <ErrorIcon fontSize="small" color="error" sx={{ mr: 1 }} />
                <Typography variant="body2" color="text.secondary">Critical</Typography>
              </Box>
              <Typography variant="h4" color="error.main">{countBySeverity('critical')}</Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card
            variant="outlined"
            sx={{ borderLeft: '4px solid', borderLeftColor: 'warning.main' }}
          >
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <ReportProblem fontSize="small" color="warning" sx={{ mr: 1 }} />
                <Typography variant="body2" color="text.secondary">High</Typography>
              </Box>
              <Typography variant="h4" color="warning.main">{countBySeverity('high')}</Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card
            variant="outlined"
            sx={{ borderLeft: '4px solid', borderLeftColor: 'info.main' }}
          >
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 0.5 }}>
                <Info fontSize="small" color="info" sx={{ mr: 1 }} />
                <Typography variant="body2" color="text.secondary">Medium</Typography>
              </Box>
              <Typography variant="h4" color="info.main">{countBySeverity('medium')}</Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* ---- Anomalies table ---- */}
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
                <TableCell>Severity</TableCell>
                <TableCell>Description</TableCell>
                <TableCell>Type</TableCell>
                <TableCell>Source</TableCell>
                <TableCell>Detected</TableCell>
                <TableCell>Status</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {anomalies.length === 0 && !loading ? (
                <TableRow>
                  <TableCell colSpan={7} align="center">
                    <Box sx={{ py: 6 }}>
                      <CheckCircle color="success" sx={{ fontSize: 48, mb: 1 }} />
                      <Typography variant="body1" color="text.secondary">
                        No anomalies detected
                      </Typography>
                      <Typography variant="body2" color="text.secondary">
                        The ML engine is monitoring your network traffic for abnormal patterns.
                      </Typography>
                    </Box>
                  </TableCell>
                </TableRow>
              ) : (
                anomalies.map((anomaly) => {
                  const severityCfg = SEVERITY_CONFIG[anomaly.severity as Severity] ?? SEVERITY_CONFIG.info;
                  return (
                    <TableRow key={anomaly.id} hover>
                      <TableCell>
                        <Chip
                          icon={severityCfg.icon}
                          label={anomaly.severity.toUpperCase()}
                          size="small"
                          color={severityCfg.color}
                        />
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" sx={{ fontWeight: 500, maxWidth: 350 }} noWrap>
                          {anomaly.description}
                        </Typography>
                        {anomaly.remediation && (
                          <Typography variant="caption" color="success.main" sx={{ display: 'block' }} noWrap>
                            {anomaly.remediation}
                          </Typography>
                        )}
                      </TableCell>
                      <TableCell>
                        <Chip label={anomaly.anomaly_type.replace(/_/g, ' ')} size="small" variant="outlined" />
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2">
                          {anomaly.source_namespace}/{anomaly.source_pod ?? '—'}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2">{formatTimestamp(anomaly.detected_at)}</Typography>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={anomaly.status}
                          size="small"
                          color={STATUS_COLORS[anomaly.status] ?? 'default'}
                        />
                      </TableCell>
                      <TableCell align="right">
                        <Tooltip title="Remediate">
                          <span>
                            <Button
                              size="small"
                              variant="outlined"
                              color="warning"
                              startIcon={<Build fontSize="small" />}
                              disabled={anomaly.status === 'remediated' || anomaly.status === 'resolved'}
                              onClick={() => {
                                setRemediatingId(anomaly.id);
                                setRemediateOpen(true);
                              }}
                            >
                              Remediate
                            </Button>
                          </span>
                        </Tooltip>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      {/* ================================================================== */}
      {/* Remediate Confirmation Dialog                                       */}
      {/* ================================================================== */}
      <Dialog open={remediateOpen} onClose={() => setRemediateOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Confirm Remediation</DialogTitle>
        <DialogContent>
          <Typography>
            This will trigger the automated remediation workflow for this anomaly.
            The system will attempt to apply corrective network policies automatically.
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            Are you sure you want to proceed?
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setRemediateOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="warning"
            onClick={handleRemediate}
            disabled={remediating}
            startIcon={remediating ? <CircularProgress size={16} /> : <Build />}
          >
            Remediate
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default Anomalies;
