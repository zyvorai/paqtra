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
  Button,
  IconButton,
  Tooltip,
  Chip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  CircularProgress,
  Alert,
  Grid,
  Card,
  CardContent,
} from '@mui/material';
import {
  Add,
  Delete,
  Refresh,
  Visibility,
  Policy as PolicyIcon,
  Close,
  Science,
} from '@mui/icons-material';
import axios from 'axios';
import { format, parseISO } from 'date-fns';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Policy {
  id: string;
  name: string;
  namespace: string;
  created_at: string;
  status: string;
}

interface PoliciesResponse {
  policies: Policy[];
  total: number;
}

interface SimulationResult {
  policy: string;
  impact: {
    flows_affected: number;
    services_impacted: number;
    risk_level: string;
    note?: string;
  };
}

const STATUS_COLORS: Record<string, 'success' | 'warning' | 'error' | 'info' | 'default'> = {
  active: 'success',
  enforcing: 'success',
  created: 'info',
  pending: 'warning',
  error: 'error',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Policies: React.FC = () => {
  // Data state
  const [policies, setPolicies] = useState<Policy[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Create dialog state
  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [newNamespace, setNewNamespace] = useState('default');
  const [newSpec, setNewSpec] = useState('{\n  "ingress": [],\n  "egress": []\n}');
  const [creating, setCreating] = useState(false);

  // Detail dialog state
  const [detailOpen, setDetailOpen] = useState(false);
  const [selectedPolicy, setSelectedPolicy] = useState<Policy | null>(null);

  // Delete dialog state
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Simulate state
  const [simulating, setSimulating] = useState(false);
  const [simulationResult, setSimulationResult] = useState<SimulationResult | null>(null);
  const [simulateOpen, setSimulateOpen] = useState(false);

  // -------------------------------------------------------------------------
  // Data fetching
  // -------------------------------------------------------------------------

  const fetchPolicies = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await axios.get<PoliciesResponse>('/api/v1/policies');
      setPolicies(response.data.policies);
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to fetch policies';
      setError(String(message));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchPolicies();
  }, [fetchPolicies]);

  // -------------------------------------------------------------------------
  // Create policy
  // -------------------------------------------------------------------------

  const handleCreate = async () => {
    if (!newName.trim()) return;
    setCreating(true);
    setError(null);
    try {
      let spec: unknown;
      try {
        spec = JSON.parse(newSpec);
      } catch {
        setError('Invalid JSON in spec field');
        setCreating(false);
        return;
      }

      await axios.post('/api/v1/policies', {
        name: newName.trim(),
        namespace: newNamespace.trim() || 'default',
        spec,
      });

      setCreateOpen(false);
      setNewName('');
      setNewNamespace('default');
      setNewSpec('{\n  "ingress": [],\n  "egress": []\n}');
      setSuccessMessage('Policy created successfully');
      fetchPolicies();
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to create policy';
      setError(String(message));
    } finally {
      setCreating(false);
    }
  };

  // -------------------------------------------------------------------------
  // Delete policy
  // -------------------------------------------------------------------------

  const handleDelete = async () => {
    if (!deletingId) return;
    setDeleting(true);
    setError(null);
    try {
      await axios.delete(`/api/v1/policies/${deletingId}`);
      setDeleteOpen(false);
      setDeletingId(null);
      setSuccessMessage('Policy deleted successfully');
      fetchPolicies();
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to delete policy';
      setError(String(message));
    } finally {
      setDeleting(false);
    }
  };

  // -------------------------------------------------------------------------
  // Simulate policy
  // -------------------------------------------------------------------------

  const handleSimulate = async () => {
    setSimulating(true);
    setError(null);
    try {
      let spec: unknown;
      try {
        spec = JSON.parse(newSpec);
      } catch {
        setError('Invalid JSON in spec field');
        setSimulating(false);
        return;
      }

      const response = await axios.post<SimulationResult>('/api/v1/policies/simulate', {
        name: newName.trim() || 'simulation-test',
        namespace: newNamespace.trim() || 'default',
        spec,
      });
      setSimulationResult(response.data);
      setSimulateOpen(true);
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Simulation failed';
      setError(String(message));
    } finally {
      setSimulating(false);
    }
  };

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

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
        <PolicyIcon sx={{ mr: 1 }} />
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Policy Management
        </Typography>
        <Tooltip title="Refresh">
          <IconButton onClick={fetchPolicies} disabled={loading} sx={{ mr: 1 }}>
            <Refresh />
          </IconButton>
        </Tooltip>
        <Button
          variant="contained"
          startIcon={<Add />}
          onClick={() => setCreateOpen(true)}
        >
          Create Policy
        </Button>
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
          <Card variant="outlined">
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Typography variant="body2" color="text.secondary">Total Policies</Typography>
              <Typography variant="h5">{policies.length}</Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card variant="outlined">
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Typography variant="body2" color="text.secondary">Active</Typography>
              <Typography variant="h5" color="success.main">
                {policies.filter((p) => p.status === 'active' || p.status === 'enforcing').length}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card variant="outlined">
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Typography variant="body2" color="text.secondary">Pending</Typography>
              <Typography variant="h5" color="warning.main">
                {policies.filter((p) => p.status === 'pending' || p.status === 'created').length}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
        <Grid item xs={6} sm={3}>
          <Card variant="outlined">
            <CardContent sx={{ py: 1.5, '&:last-child': { pb: 1.5 } }}>
              <Typography variant="body2" color="text.secondary">Errors</Typography>
              <Typography variant="h5" color="error.main">
                {policies.filter((p) => p.status === 'error').length}
              </Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* ---- Policies table ---- */}
      <Paper sx={{ width: '100%', overflow: 'hidden' }}>
        {loading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
            <CircularProgress size={28} />
          </Box>
        )}

        <TableContainer>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Name</TableCell>
                <TableCell>Namespace</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Created</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {policies.length === 0 && !loading ? (
                <TableRow>
                  <TableCell colSpan={5} align="center">
                    <Typography variant="body2" color="text.secondary" sx={{ py: 4 }}>
                      No policies found. Create a new policy to get started.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                policies.map((policy) => (
                  <TableRow key={policy.id} hover>
                    <TableCell>
                      <Typography variant="body2" sx={{ fontWeight: 500 }}>
                        {policy.name}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Chip label={policy.namespace} size="small" variant="outlined" />
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={policy.status}
                        size="small"
                        color={STATUS_COLORS[policy.status] ?? 'default'}
                      />
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">{formatTimestamp(policy.created_at)}</Typography>
                    </TableCell>
                    <TableCell align="right">
                      <Tooltip title="View details">
                        <IconButton
                          size="small"
                          onClick={() => {
                            setSelectedPolicy(policy);
                            setDetailOpen(true);
                          }}
                        >
                          <Visibility fontSize="small" />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title="Delete">
                        <IconButton
                          size="small"
                          color="error"
                          onClick={() => {
                            setDeletingId(policy.id);
                            setDeleteOpen(true);
                          }}
                        >
                          <Delete fontSize="small" />
                        </IconButton>
                      </Tooltip>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      {/* ================================================================== */}
      {/* Create Policy Dialog                                               */}
      {/* ================================================================== */}
      <Dialog open={createOpen} onClose={() => setCreateOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>
          Create Network Policy
          <IconButton
            onClick={() => setCreateOpen(false)}
            sx={{ position: 'absolute', right: 8, top: 8 }}
          >
            <Close />
          </IconButton>
        </DialogTitle>
        <DialogContent dividers>
          <Grid container spacing={2} sx={{ mt: 0 }}>
            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label="Policy Name"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                required
                placeholder="my-network-policy"
              />
            </Grid>
            <Grid item xs={12} sm={6}>
              <TextField
                fullWidth
                label="Namespace"
                value={newNamespace}
                onChange={(e) => setNewNamespace(e.target.value)}
                placeholder="default"
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Policy Spec (JSON)"
                value={newSpec}
                onChange={(e) => setNewSpec(e.target.value)}
                multiline
                minRows={10}
                maxRows={20}
                InputProps={{
                  sx: { fontFamily: 'monospace', fontSize: 13 },
                }}
              />
            </Grid>
          </Grid>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={handleSimulate}
            startIcon={simulating ? <CircularProgress size={16} /> : <Science />}
            disabled={simulating}
          >
            Simulate
          </Button>
          <Button onClick={() => setCreateOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={handleCreate}
            disabled={creating || !newName.trim()}
            startIcon={creating ? <CircularProgress size={16} /> : <Add />}
          >
            Create
          </Button>
        </DialogActions>
      </Dialog>

      {/* ================================================================== */}
      {/* Policy Detail Dialog (YAML display)                                */}
      {/* ================================================================== */}
      <Dialog open={detailOpen} onClose={() => setDetailOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>
          Policy Details
          <IconButton
            onClick={() => setDetailOpen(false)}
            sx={{ position: 'absolute', right: 8, top: 8 }}
          >
            <Close />
          </IconButton>
        </DialogTitle>
        <DialogContent dividers>
          {selectedPolicy && (
            <Box>
              <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                Metadata
              </Typography>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body2"><strong>ID:</strong> {selectedPolicy.id}</Typography>
                <Typography variant="body2"><strong>Name:</strong> {selectedPolicy.name}</Typography>
                <Typography variant="body2"><strong>Namespace:</strong> {selectedPolicy.namespace}</Typography>
                <Typography variant="body2"><strong>Status:</strong> {selectedPolicy.status}</Typography>
                <Typography variant="body2"><strong>Created:</strong> {formatTimestamp(selectedPolicy.created_at)}</Typography>
              </Box>

              <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                YAML Representation
              </Typography>
              <Paper
                variant="outlined"
                sx={{
                  p: 2,
                  fontFamily: 'monospace',
                  fontSize: 13,
                  whiteSpace: 'pre-wrap',
                  bgcolor: 'background.default',
                  maxHeight: 300,
                  overflow: 'auto',
                }}
              >
{`apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: ${selectedPolicy.name}
  namespace: ${selectedPolicy.namespace}
spec:
  endpointSelector: {}
  ingress: []
  egress: []`}
              </Paper>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDetailOpen(false)}>Close</Button>
        </DialogActions>
      </Dialog>

      {/* ================================================================== */}
      {/* Delete Confirmation Dialog                                         */}
      {/* ================================================================== */}
      <Dialog open={deleteOpen} onClose={() => setDeleteOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Delete Policy</DialogTitle>
        <DialogContent>
          <Typography>
            Are you sure you want to delete this policy? This action cannot be undone.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteOpen(false)}>Cancel</Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleDelete}
            disabled={deleting}
            startIcon={deleting ? <CircularProgress size={16} /> : <Delete />}
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>

      {/* ================================================================== */}
      {/* Simulation Result Dialog                                           */}
      {/* ================================================================== */}
      <Dialog open={simulateOpen} onClose={() => setSimulateOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>
          Simulation Result
          <IconButton
            onClick={() => setSimulateOpen(false)}
            sx={{ position: 'absolute', right: 8, top: 8 }}
          >
            <Close />
          </IconButton>
        </DialogTitle>
        <DialogContent dividers>
          {simulationResult && (
            <Grid container spacing={2}>
              <Grid item xs={12}>
                <Typography variant="subtitle2" color="text.secondary">
                  Policy: {simulationResult.policy}
                </Typography>
              </Grid>
              <Grid item xs={6}>
                <Card variant="outlined">
                  <CardContent>
                    <Typography variant="body2" color="text.secondary">Flows Affected</Typography>
                    <Typography variant="h5">{simulationResult.impact.flows_affected}</Typography>
                  </CardContent>
                </Card>
              </Grid>
              <Grid item xs={6}>
                <Card variant="outlined">
                  <CardContent>
                    <Typography variant="body2" color="text.secondary">Services Impacted</Typography>
                    <Typography variant="h5">{simulationResult.impact.services_impacted}</Typography>
                  </CardContent>
                </Card>
              </Grid>
              <Grid item xs={12}>
                <Typography variant="body2">
                  <strong>Risk Level:</strong>{' '}
                  <Chip
                    label={simulationResult.impact.risk_level}
                    size="small"
                    color={
                      simulationResult.impact.risk_level === 'low'
                        ? 'success'
                        : simulationResult.impact.risk_level === 'high'
                          ? 'error'
                          : 'warning'
                    }
                  />
                </Typography>
              </Grid>
              {simulationResult.impact.note && (
                <Grid item xs={12}>
                  <Alert severity="info">{simulationResult.impact.note}</Alert>
                </Grid>
              )}
            </Grid>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSimulateOpen(false)}>Close</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default Policies;
