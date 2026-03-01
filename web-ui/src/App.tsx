import React, { Suspense } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { CircularProgress, Box } from '@mui/material';
import { Provider } from 'react-redux';
import { store } from './store';

// Lazy-loaded pages (code-split per route)
const Dashboard = React.lazy(() => import('./pages/Dashboard'));
const Flows = React.lazy(() => import('./pages/Flows'));
const Topology = React.lazy(() => import('./pages/Topology'));
const Policies = React.lazy(() => import('./pages/Policies'));
const Anomalies = React.lazy(() => import('./pages/Anomalies'));
const Compliance = React.lazy(() => import('./pages/Compliance'));
const Settings = React.lazy(() => import('./pages/Settings'));

// Layout & ErrorBoundary (loaded eagerly - small)
import Layout from './components/Layout';
import ErrorBoundary from './components/ErrorBoundary';

// Loading fallback
const PageLoader = () => (
  <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '60vh' }}>
    <CircularProgress />
  </Box>
);

// Create dark theme
const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#3f51b5',
    },
    secondary: {
      main: '#f50057',
    },
    background: {
      default: '#0a1929',
      paper: '#132f4c',
    },
  },
  typography: {
    fontFamily: '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
  },
});

const App: React.FC = () => {
  return (
    <Provider store={store}>
      <ThemeProvider theme={darkTheme}>
        <CssBaseline />
        <ErrorBoundary>
          <Router>
            <Layout>
              <Suspense fallback={<PageLoader />}>
                <Routes>
                  <Route path="/" element={<Dashboard />} />
                  <Route path="/flows" element={<Flows />} />
                  <Route path="/topology" element={<Topology />} />
                  <Route path="/policies" element={<Policies />} />
                  <Route path="/anomalies" element={<Anomalies />} />
                  <Route path="/compliance" element={<Compliance />} />
                  <Route path="/settings" element={<Settings />} />
                </Routes>
              </Suspense>
            </Layout>
          </Router>
        </ErrorBoundary>
      </ThemeProvider>
    </Provider>
  );
};

export default App;
