import React from 'react';
import { Box, Typography } from '@mui/material';

const Policies: React.FC = () => {
  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>Policy Management</Typography>
      <Typography>Network policy editor and manager</Typography>
    </Box>
  );
};

export default Policies;
