import React, { Suspense, useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import MainLayout from './components/MainLayout';
import ErrorBoundary from './components/ErrorBoundary';
import LoadingSpinner from './components/LoadingSpinner';
import LoginPage from './components/LoginPage';
import { ToastProvider } from './components/Toast';
import { useAuthStore } from './stores/authStore';
import { useThemeStore } from './stores/themeStore';

// Lazy-loaded views
const Dashboard = React.lazy(() => import('./views/Dashboard'));
const Flows = React.lazy(() => import('./views/Flows'));
const Topology = React.lazy(() => import('./views/Topology'));
const Policies = React.lazy(() => import('./views/Policies'));
const Anomalies = React.lazy(() => import('./views/Anomalies'));
const Compliance = React.lazy(() => import('./views/Compliance'));
const AutoPolicy = React.lazy(() => import('./views/AutoPolicy'));
const Chaos = React.lazy(() => import('./views/Chaos'));
const Canary = React.lazy(() => import('./views/Canary'));
const Events = React.lazy(() => import('./views/Events'));
const Endpoints = React.lazy(() => import('./views/Endpoints'));
const Nodes = React.lazy(() => import('./views/Nodes'));
const Replay = React.lazy(() => import('./views/Replay'));
const Healer = React.lazy(() => import('./views/Healer'));
const RootCause = React.lazy(() => import('./views/RootCause'));
const MultiCluster = React.lazy(() => import('./views/MultiCluster'));
const Heatmap = React.lazy(() => import('./views/Heatmap'));
const ServiceDeps = React.lazy(() => import('./views/ServiceDeps'));
const SecurityDash = React.lazy(() => import('./views/SecurityDash'));
const EbpfProfiler = React.lazy(() => import('./views/EbpfProfiler'));
const MetricsDash = React.lazy(() => import('./views/MetricsDash'));
const HostInfo = React.lazy(() => import('./views/HostInfo'));
const PolicyTemplates = React.lazy(() => import('./views/PolicyTemplates'));
const Diagnostics = React.lazy(() => import('./views/Diagnostics'));
const AuditLog = React.lazy(() => import('./views/AuditLog'));
const Alerts = React.lazy(() => import('./views/Alerts'));
const ServiceMap = React.lazy(() => import('./views/ServiceMap'));
const PacketCapture = React.lazy(() => import('./views/PacketCapture'));
const DnsMonitor = React.lazy(() => import('./views/DnsMonitor'));
const Identities = React.lazy(() => import('./views/Identities'));
const ClusterMesh = React.lazy(() => import('./views/ClusterMesh'));
const BgpPeering = React.lazy(() => import('./views/BgpPeering'));
const Bandwidth = React.lazy(() => import('./views/Bandwidth'));
const CostAnalytics = React.lazy(() => import('./views/CostAnalytics'));
const Forecasting = React.lazy(() => import('./views/Forecasting'));
const EncryptionView = React.lazy(() => import('./views/Encryption'));
const LoadBalancerView = React.lazy(() => import('./views/LoadBalancer'));
const IngressGateway = React.lazy(() => import('./views/IngressGateway'));
const IPAMView = React.lazy(() => import('./views/IPAM'));
const LatencyAnalysis = React.lazy(() => import('./views/LatencyAnalysis'));
const TrafficMirror = React.lazy(() => import('./views/TrafficMirror'));
const ClusterHealthView = React.lazy(() => import('./views/ClusterHealth'));
const RBACVisualizer = React.lazy(() => import('./views/RBACVisualizer'));
const NetworkIfaces = React.lazy(() => import('./views/NetworkIfaces'));
const TroubleshootView = React.lazy(() => import('./views/Troubleshoot'));
const WireGuardPeers = React.lazy(() => import('./views/WireGuardPeers'));
const CiliumStatusView = React.lazy(() => import('./views/CiliumStatus'));
const PolicyEditorView = React.lazy(() => import('./views/PolicyEditor'));
const FlowExporter = React.lazy(() => import('./views/FlowExporter'));
const SLODashboard = React.lazy(() => import('./views/SLODashboard'));
const IncidentTimeline = React.lazy(() => import('./views/IncidentTimeline'));
const ChangeLogView = React.lazy(() => import('./views/ChangeLog'));
const NodeDrainView = React.lazy(() => import('./views/NodeDrain'));
const PodSecurityView = React.lazy(() => import('./views/PodSecurity'));
const EgressGatewayView = React.lazy(() => import('./views/EgressGateway'));
const ServiceMeshViewComp = React.lazy(() => import('./views/ServiceMeshView'));
const KubeProxyReplacement = React.lazy(() => import('./views/KubeProxyReplacement'));
const Settings = React.lazy(() => import('./views/Settings'));
const NotFound = React.lazy(() => import('./views/NotFound'));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false },
  },
});

const App: React.FC = () => {
  const { authRequired, checkSession } = useAuthStore();
  const isDark = useThemeStore((s) => s.isDark);

  useEffect(() => {
    document.documentElement.classList.toggle('dark', isDark);
    document.documentElement.classList.toggle('light-theme', !isDark);
  }, [isDark]);

  useEffect(() => {
    checkSession();
  }, [checkSession]);

  if (authRequired) {
    return <LoginPage />;
  }

  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <ErrorBoundary>
          <Router>
            <Suspense fallback={<LoadingSpinner size="lg" text="Loading..." fullScreen />}>
              <Routes>
                <Route element={<MainLayout />}>
                  <Route path="/" element={<Dashboard />} />
                  <Route path="/flows" element={<Flows />} />
                  <Route path="/topology" element={<Topology />} />
                  <Route path="/policies" element={<Policies />} />
                  <Route path="/anomalies" element={<Anomalies />} />
                  <Route path="/compliance" element={<Compliance />} />
                  <Route path="/autopolicy" element={<AutoPolicy />} />
                  <Route path="/chaos" element={<Chaos />} />
                  <Route path="/canary" element={<Canary />} />
                  <Route path="/events" element={<Events />} />
                  <Route path="/endpoints" element={<Endpoints />} />
                  <Route path="/nodes" element={<Nodes />} />
                  <Route path="/replay" element={<Replay />} />
                  <Route path="/healer" element={<Healer />} />
                  <Route path="/rootcause" element={<RootCause />} />
                  <Route path="/multicluster" element={<MultiCluster />} />
                  <Route path="/heatmap" element={<Heatmap />} />
                  <Route path="/dependencies" element={<ServiceDeps />} />
                  <Route path="/security" element={<SecurityDash />} />
                  <Route path="/ebpf" element={<EbpfProfiler />} />
                  <Route path="/metrics" element={<MetricsDash />} />
                  <Route path="/host" element={<HostInfo />} />
                  <Route path="/templates" element={<PolicyTemplates />} />
                  <Route path="/diagnostics" element={<Diagnostics />} />
                  <Route path="/audit" element={<AuditLog />} />
                  <Route path="/alerts" element={<Alerts />} />
                  <Route path="/servicemap" element={<ServiceMap />} />
                  <Route path="/capture" element={<PacketCapture />} />
                  <Route path="/dns" element={<DnsMonitor />} />
                  <Route path="/identities" element={<Identities />} />
                  <Route path="/clustermesh" element={<ClusterMesh />} />
                  <Route path="/bgp" element={<BgpPeering />} />
                  <Route path="/bandwidth" element={<Bandwidth />} />
                  <Route path="/costs" element={<CostAnalytics />} />
                  <Route path="/forecast" element={<Forecasting />} />
                  <Route path="/encryption" element={<EncryptionView />} />
                  <Route path="/loadbalancer" element={<LoadBalancerView />} />
                  <Route path="/ingress" element={<IngressGateway />} />
                  <Route path="/ipam" element={<IPAMView />} />
                  <Route path="/latency" element={<LatencyAnalysis />} />
                  <Route path="/mirror" element={<TrafficMirror />} />
                  <Route path="/clusterhealth" element={<ClusterHealthView />} />
                  <Route path="/rbac" element={<RBACVisualizer />} />
                  <Route path="/interfaces" element={<NetworkIfaces />} />
                  <Route path="/troubleshoot" element={<TroubleshootView />} />
                  <Route path="/wireguard" element={<WireGuardPeers />} />
                  <Route path="/cilium-status" element={<CiliumStatusView />} />
                  <Route path="/policy-editor" element={<PolicyEditorView />} />
                  <Route path="/flow-export" element={<FlowExporter />} />
                  <Route path="/slo" element={<SLODashboard />} />
                  <Route path="/incidents" element={<IncidentTimeline />} />
                  <Route path="/changelog" element={<ChangeLogView />} />
                  <Route path="/node-drain" element={<NodeDrainView />} />
                  <Route path="/pod-security" element={<PodSecurityView />} />
                  <Route path="/egress" element={<EgressGatewayView />} />
                  <Route path="/service-mesh" element={<ServiceMeshViewComp />} />
                  <Route path="/kpr" element={<KubeProxyReplacement />} />
                  <Route path="/settings" element={<Settings />} />
                  <Route path="*" element={<NotFound />} />
                </Route>
              </Routes>
            </Suspense>
          </Router>
        </ErrorBoundary>
      </ToastProvider>
    </QueryClientProvider>
  );
};

export default App;
