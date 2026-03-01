import React from 'react';
import { Box, Typography, Button, Paper } from '@mui/material';
import { SearchOff } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

const NotFound: React.FC = () => {
  const navigate = useNavigate();

  return (
    <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh', p: 3 }}>
      <Paper sx={{ p: 4, maxWidth: 420, textAlign: 'center' }}>
        <SearchOff sx={{ fontSize: 64, color: 'text.secondary', mb: 2 }} />
        <Typography variant="h4" gutterBottom>404</Typography>
        <Typography variant="body1" color="text.secondary" sx={{ mb: 3 }}>
          Page not found. The page you are looking for does not exist.
        </Typography>
        <Button variant="contained" onClick={() => navigate('/')}>
          Go to Dashboard
        </Button>
      </Paper>
    </Box>
  );
};

export default NotFound;
