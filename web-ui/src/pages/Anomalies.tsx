import React from 'react';
import { Box, Typography } from '@mui/material';

const Anomalies: React.FC = () => {
  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>Anomaly Detection</Typography>
      <Typography>AI/ML-powered anomaly detection dashboard</Typography>
    </Box>
  );
};

export default Anomalies;
