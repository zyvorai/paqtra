import React from 'react';
import { Box, Typography } from '@mui/material';

const Compliance: React.FC = () => {
  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>Security & Compliance</Typography>
      <Typography>PCI-DSS, SOC2, HIPAA, GDPR compliance dashboards</Typography>
    </Box>
  );
};

export default Compliance;
