import React from 'react';
import { Box, Typography } from '@mui/material';

const Topology: React.FC = () => {
  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>Network Topology</Typography>
      <Typography>Network graph visualization will be rendered here using D3.js/Cytoscape</Typography>
    </Box>
  );
};

export default Topology;
