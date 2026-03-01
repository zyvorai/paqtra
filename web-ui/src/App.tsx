import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { Provider } from 'react-redux';
import { store } from './store';

// Pages
import Dashboard from './pages/Dashboard';
import Flows from './pages/Flows';
import Topology from './pages/Topology';
import Policies from './pages/Policies';
import Anomalies from './pages/Anomalies';
import Compliance from './pages/Compliance';
import Settings from './pages/Settings';

// Layout
import Layout from './components/Layout';
import ErrorBoundary from './components/ErrorBoundary';

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
              <Routes>
                <Route path="/" element={<Dashboard />} />
                <Route path="/flows" element={<Flows />} />
                <Route path="/topology" element={<Topology />} />
                <Route path="/policies" element={<Policies />} />
                <Route path="/anomalies" element={<Anomalies />} />
                <Route path="/compliance" element={<Compliance />} />
                <Route path="/settings" element={<Settings />} />
              </Routes>
            </Layout>
          </Router>
        </ErrorBoundary>
      </ThemeProvider>
    </Provider>
  );
};

export default App;
