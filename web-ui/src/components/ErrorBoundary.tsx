import { Component, ErrorInfo, ReactNode } from 'react';
import { Box, Typography, Button, Paper } from '@mui/material';
import { ErrorOutline } from '@mui/icons-material';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught:', error, errorInfo);
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      const isChunkLoadError =
        this.state.error?.name === 'ChunkLoadError' ||
        this.state.error?.message?.includes('Loading chunk') ||
        this.state.error?.message?.includes('Failed to fetch dynamically imported module');

      return (
        <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh', p: 3 }}>
          <Paper sx={{ p: 4, maxWidth: 500, textAlign: 'center' }}>
            <ErrorOutline color="error" sx={{ fontSize: 64, mb: 2 }} />
            <Typography variant="h5" gutterBottom>
              {isChunkLoadError ? 'Failed to load page' : 'Something went wrong'}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              {isChunkLoadError
                ? 'A newer version may be available, or there was a network issue. Try again or reload the page.'
                : (this.state.error?.message || 'An unexpected error occurred.')}
            </Typography>
            <Box sx={{ display: 'flex', gap: 2, justifyContent: 'center' }}>
              <Button variant="outlined" onClick={this.handleRetry}>
                Try Again
              </Button>
              <Button variant="contained" onClick={() => window.location.reload()}>
                Reload Page
              </Button>
            </Box>
          </Paper>
        </Box>
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;
