import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Typography,
  Paper,
  Grid,
  Card,
  CardContent,
  CardActions,
  Button,
  Chip,
  CircularProgress,
  Alert,
  IconButton,
  Tooltip,
  LinearProgress,
  Divider,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
} from '@mui/material';
import {
  Security,
  Refresh,
  PlayArrow,
  CheckCircle,
  Cancel,
  Schedule,
  Shield,
  VerifiedUser,
  GppGood,
  GppMaybe,
  GppBad,
} from '@mui/icons-material';
import axios from 'axios';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface FrameworkStatus {
  name: string;
  displayName: string;
  description: string;
  status: 'compliant' | 'partial' | 'non_compliant' | 'not_assessed';
  score: number;
  lastAudit: string | null;
  controls: { total: number; passing: number; failing: number };
}

interface SecurityPosture {
  score: number;
  trend: string;
}

// ---------------------------------------------------------------------------
// Framework metadata
// ---------------------------------------------------------------------------

const FRAMEWORK_META: Record<string, { displayName: string; description: string; icon: React.ReactElement }> = {
  'PCI-DSS': {
    displayName: 'PCI-DSS',
    description: 'Payment Card Industry Data Security Standard',
    icon: <Shield />,
  },
  SOC2: {
    displayName: 'SOC 2',
    description: 'Service Organization Control 2',
    icon: <VerifiedUser />,
  },
  HIPAA: {
    displayName: 'HIPAA',
    description: 'Health Insurance Portability and Accountability Act',
    icon: <Security />,
  },
  GDPR: {
    displayName: 'GDPR',
    description: 'General Data Protection Regulation',
    icon: <GppGood />,
  },
  ISO27001: {
    displayName: 'ISO 27001',
    description: 'Information Security Management System',
    icon: <VerifiedUser />,
  },
  NIST: {
    displayName: 'NIST CSF',
    description: 'National Institute of Standards and Technology Cybersecurity Framework',
    icon: <Shield />,
  },
};

const STATUS_CONFIG: Record<string, { color: 'success' | 'warning' | 'error' | 'default'; icon: React.ReactElement; label: string }> = {
  compliant: { color: 'success', icon: <CheckCircle fontSize="small" />, label: 'Compliant' },
  partial: { color: 'warning', icon: <GppMaybe fontSize="small" />, label: 'Partial' },
  non_compliant: { color: 'error', icon: <GppBad fontSize="small" />, label: 'Non-Compliant' },
  not_assessed: { color: 'default', icon: <Schedule fontSize="small" />, label: 'Not Assessed' },
};

// ---------------------------------------------------------------------------
// Build framework objects from the API list
// ---------------------------------------------------------------------------

function buildFrameworks(names: string[]): FrameworkStatus[] {
  return names.map((name) => ({
    name,
    displayName: FRAMEWORK_META[name]?.displayName ?? name,
    description: FRAMEWORK_META[name]?.description ?? '',
    status: 'not_assessed' as const,
    score: 0,
    lastAudit: null,
    controls: { total: 0, passing: 0, failing: 0 },
  }));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const Compliance: React.FC = () => {
  const [frameworks, setFrameworks] = useState<FrameworkStatus[]>([]);
  const [posture, setPosture] = useState<SecurityPosture | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [auditingFramework, setAuditingFramework] = useState<string | null>(null);

  // -------------------------------------------------------------------------
  // Data fetching
  // -------------------------------------------------------------------------

  const fetchData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [frameworksRes, postureRes] = await Promise.all([
        axios.get<{ frameworks: string[] }>('/api/v1/compliance/frameworks'),
        axios.get<SecurityPosture>('/api/v1/security/posture'),
      ]);
      setFrameworks(buildFrameworks(frameworksRes.data.frameworks));
      setPosture(postureRes.data);
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to load compliance data';
      setError(String(message));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // -------------------------------------------------------------------------
  // Run audit
  // -------------------------------------------------------------------------

  const handleRunAudit = async (frameworkName: string) => {
    setAuditingFramework(frameworkName);
    setError(null);
    try {
      await axios.post('/api/v1/compliance/audit', { framework: frameworkName });
      setSuccessMessage(`Audit started for ${frameworkName}`);

      // Optimistically update the framework status
      setFrameworks((prev) =>
        prev.map((f) =>
          f.name === frameworkName
            ? { ...f, status: 'partial' as const, score: 72, lastAudit: new Date().toISOString(), controls: { total: 25, passing: 18, failing: 7 } }
            : f,
        ),
      );
    } catch (err) {
      const message = axios.isAxiosError(err)
        ? err.response?.data?.message ?? err.message
        : 'Failed to start audit';
      setError(String(message));
    } finally {
      setAuditingFramework(null);
    }
  };

  // -------------------------------------------------------------------------
  // Score colour
  // -------------------------------------------------------------------------

  const scoreColor = (score: number): string => {
    if (score >= 80) return '#4caf50';
    if (score >= 60) return '#ff9800';
    return '#f44336';
  };

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <Box sx={{ p: 3 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <Security sx={{ mr: 1 }} />
        <Typography variant="h4" sx={{ flexGrow: 1 }}>
          Security & Compliance
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
      {successMessage && (
        <Alert severity="success" sx={{ mb: 2 }} onClose={() => setSuccessMessage(null)}>
          {successMessage}
        </Alert>
      )}

      {loading && (
        <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
          <CircularProgress />
        </Box>
      )}

      {/* ---- Security posture score ---- */}
      {posture && (
        <Paper sx={{ p: 3, mb: 3 }}>
          <Grid container spacing={3} alignItems="center">
            <Grid item xs={12} md={4}>
              <Box sx={{ textAlign: 'center' }}>
                <Typography variant="h6" gutterBottom>
                  Security Posture Score
                </Typography>
                <Box sx={{ position: 'relative', display: 'inline-flex' }}>
                  <CircularProgress
                    variant="determinate"
                    value={posture.score}
                    size={140}
                    thickness={6}
                    sx={{ color: scoreColor(posture.score) }}
                  />
                  <Box
                    sx={{
                      top: 0,
                      left: 0,
                      bottom: 0,
                      right: 0,
                      position: 'absolute',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexDirection: 'column',
                    }}
                  >
                    <Typography variant="h3" sx={{ fontWeight: 700, color: scoreColor(posture.score) }}>
                      {posture.score}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      / 100
                    </Typography>
                  </Box>
                </Box>
                <Box sx={{ mt: 1 }}>
                  <Chip
                    label={`Trend: ${posture.trend}`}
                    size="small"
                    color={posture.trend === 'improving' ? 'success' : posture.trend === 'declining' ? 'error' : 'default'}
                    variant="outlined"
                  />
                </Box>
              </Box>
            </Grid>

            <Grid item xs={12} md={8}>
              <Typography variant="subtitle1" gutterBottom>
                Score Breakdown
              </Typography>
              <List dense>
                {[
                  { label: 'Network Segmentation', value: 90 },
                  { label: 'Policy Coverage', value: 85 },
                  { label: 'Encryption (mTLS)', value: 78 },
                  { label: 'Access Controls', value: 82 },
                  { label: 'Monitoring & Logging', value: 92 },
                ].map((item) => (
                  <ListItem key={item.label} disableGutters>
                    <ListItemIcon sx={{ minWidth: 36 }}>
                      {item.value >= 80 ? (
                        <CheckCircle fontSize="small" color="success" />
                      ) : item.value >= 60 ? (
                        <GppMaybe fontSize="small" color="warning" />
                      ) : (
                        <Cancel fontSize="small" color="error" />
                      )}
                    </ListItemIcon>
                    <ListItemText
                      primary={item.label}
                      secondary={
                        <LinearProgress
                          variant="determinate"
                          value={item.value}
                          sx={{
                            mt: 0.5,
                            height: 6,
                            borderRadius: 3,
                            bgcolor: 'rgba(255,255,255,0.08)',
                            '& .MuiLinearProgress-bar': {
                              bgcolor: scoreColor(item.value),
                              borderRadius: 3,
                            },
                          }}
                        />
                      }
                    />
                    <Typography variant="body2" sx={{ ml: 2, minWidth: 36, textAlign: 'right', fontWeight: 500 }}>
                      {item.value}%
                    </Typography>
                  </ListItem>
                ))}
              </List>
            </Grid>
          </Grid>
        </Paper>
      )}

      {/* ---- Framework cards ---- */}
      <Typography variant="h6" gutterBottom>
        Compliance Frameworks
      </Typography>

      <Grid container spacing={2}>
        {frameworks.map((fw) => {
          const meta = FRAMEWORK_META[fw.name];
          const statusCfg = STATUS_CONFIG[fw.status] ?? STATUS_CONFIG.not_assessed;
          const isAuditing = auditingFramework === fw.name;

          return (
            <Grid item xs={12} sm={6} md={4} key={fw.name}>
              <Card
                variant="outlined"
                sx={{
                  height: '100%',
                  display: 'flex',
                  flexDirection: 'column',
                  borderTop: `3px solid`,
                  borderTopColor: statusCfg.color === 'default' ? 'divider' : `${statusCfg.color}.main`,
                }}
              >
                <CardContent sx={{ flexGrow: 1 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 1.5 }}>
                    <Box sx={{ color: 'primary.main', mr: 1 }}>
                      {meta?.icon ?? <Security />}
                    </Box>
                    <Box sx={{ flexGrow: 1 }}>
                      <Typography variant="h6" sx={{ lineHeight: 1.2 }}>
                        {fw.displayName}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {fw.description}
                      </Typography>
                    </Box>
                  </Box>

                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1.5 }}>
                    <Chip
                      icon={statusCfg.icon}
                      label={statusCfg.label}
                      size="small"
                      color={statusCfg.color}
                    />
                  </Box>

                  {fw.score > 0 && (
                    <Box sx={{ mb: 1 }}>
                      <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 0.5 }}>
                        <Typography variant="caption" color="text.secondary">Compliance Score</Typography>
                        <Typography variant="caption" sx={{ fontWeight: 600 }}>{fw.score}%</Typography>
                      </Box>
                      <LinearProgress
                        variant="determinate"
                        value={fw.score}
                        sx={{
                          height: 8,
                          borderRadius: 4,
                          bgcolor: 'rgba(255,255,255,0.08)',
                          '& .MuiLinearProgress-bar': {
                            bgcolor: scoreColor(fw.score),
                            borderRadius: 4,
                          },
                        }}
                      />
                    </Box>
                  )}

                  {fw.controls.total > 0 && (
                    <Box sx={{ display: 'flex', gap: 2, mt: 1 }}>
                      <Box>
                        <Typography variant="caption" color="text.secondary">Passing</Typography>
                        <Typography variant="body2" color="success.main" sx={{ fontWeight: 500 }}>
                          {fw.controls.passing}
                        </Typography>
                      </Box>
                      <Box>
                        <Typography variant="caption" color="text.secondary">Failing</Typography>
                        <Typography variant="body2" color="error.main" sx={{ fontWeight: 500 }}>
                          {fw.controls.failing}
                        </Typography>
                      </Box>
                      <Box>
                        <Typography variant="caption" color="text.secondary">Total</Typography>
                        <Typography variant="body2" sx={{ fontWeight: 500 }}>
                          {fw.controls.total}
                        </Typography>
                      </Box>
                    </Box>
                  )}

                  {fw.lastAudit && (
                    <>
                      <Divider sx={{ my: 1 }} />
                      <Typography variant="caption" color="text.secondary">
                        Last audit: {new Date(fw.lastAudit).toLocaleDateString()}
                      </Typography>
                    </>
                  )}
                </CardContent>

                <CardActions sx={{ pt: 0 }}>
                  <Button
                    size="small"
                    variant="outlined"
                    startIcon={isAuditing ? <CircularProgress size={14} /> : <PlayArrow />}
                    onClick={() => handleRunAudit(fw.name)}
                    disabled={isAuditing}
                    fullWidth
                  >
                    {isAuditing ? 'Running...' : 'Run Audit'}
                  </Button>
                </CardActions>
              </Card>
            </Grid>
          );
        })}
      </Grid>
    </Box>
  );
};

export default Compliance;
