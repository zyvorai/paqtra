import axios, { AxiosError, InternalAxiosRequestConfig } from 'axios';

/** Settings stored in localStorage by the Settings page */
interface AppSettings {
  apiBaseUrl: string;
  hubbleAddress: string;
  refreshInterval: number;
  darkMode: boolean;
}

const DEFAULT_SETTINGS: AppSettings = {
  apiBaseUrl: '/api/v1',
  hubbleAddress: 'localhost:4245',
  refreshInterval: 5,
  darkMode: true,
};

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem('cilium-vision-settings');
    if (raw) return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    // ignore
  }
  return DEFAULT_SETTINGS;
}

/**
 * Shared Axios instance with:
 * - Base URL from localStorage settings (or /api/v1)
 * - Authorization header injection
 * - Response error logging
 */
const api = axios.create({
  baseURL: loadSettings().apiBaseUrl,
  timeout: 15_000,
  headers: { 'Content-Type': 'application/json' },
});

// --- Request interceptor ---------------------------------------------------
api.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  // Attach JWT token if available
  const token = sessionStorage.getItem('cilium-vision-token');
  if (token && config.headers) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// --- Response interceptor ---------------------------------------------------
api.interceptors.response.use(
  (response) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      // Token expired or invalid – clear and let the UI handle it
      sessionStorage.removeItem('cilium-vision-token');
      // In production, send to observability service (e.g., Sentry, Datadog)
      console.warn('[api] Unauthorized – token cleared');
    }
    return Promise.reject(error);
  },
);

export default api;

// ─── Typed API helpers ──────────────────────────────────────────────────────

// Flows
export interface FlowEndpoint {
  namespace: string;
  pod: string;
  ip: string;
}

export interface Flow {
  id: string;
  timestamp: string;
  source: FlowEndpoint;
  destination: FlowEndpoint;
  verdict: string;
  protocol: string;
  port: number;
}

export interface FlowStats {
  total_flows: number;
  forwarded: number;
  dropped: number;
  requests_per_second: number;
  avg_latency_ms: number;
}

export const fetchFlows = (params?: {
  namespace?: string;
  verdict?: string;
  limit?: number;
  offset?: number;
}) => api.get<{ flows: Flow[]; total: number }>('/flows', { params });

export const fetchFlowStats = () => api.get<FlowStats>('/flows/stats');

// Policies
export interface Policy {
  id: string;
  name: string;
  namespace: string;
  created_at: string;
  status: string;
}

export const fetchPolicies = () =>
  api.get<{ policies: Policy[] }>('/policies');

export const createPolicy = (body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.post('/policies', body);

export const deletePolicy = (id: string) => api.delete(`/policies/${id}`);

export const simulatePolicy = (body: {
  name: string;
  namespace: string;
  spec: unknown;
}) => api.post('/policies/simulate', body);

// Anomalies
export interface Anomaly {
  id: string;
  title: string;
  description: string;
  severity: string;
  category: string;
  source: string;
  source_namespace: string;
  source_pod: string;
  detected_at: string;
  status: string;
}

export const fetchAnomalies = () =>
  api.get<{ anomalies: Anomaly[]; total: number }>('/anomalies');

export const remediateAnomaly = (id: string) =>
  api.post(`/anomalies/${id}/remediate`);

// Compliance
export const fetchFrameworks = () =>
  api.get<{ frameworks: string[] }>('/compliance/frameworks');

export const runAudit = (framework: string) =>
  api.post('/compliance/audit', { framework });

export const fetchSecurityPosture = () =>
  api.get<{ score: number; trend: string }>('/security/posture');

// Modules
export const generateAutopolicy = (body: unknown) =>
  api.post('/modules/autopolicy/generate', body);

export const fetchChaosExperiments = () =>
  api.get('/modules/chaos/experiments');

export const runChaosExperiment = (body: unknown) =>
  api.post('/modules/chaos/run', body);

export const fetchCanaryStatus = (id: string) =>
  api.get(`/modules/canary/${id}`);

// Health
export const checkHealth = () => api.get('/health');
export const checkReady = () => api.get('/ready');
