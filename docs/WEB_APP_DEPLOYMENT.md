# Cilium Vision Web Application - Deployment Guide

## Quick Start

### Prerequisites
- Kubernetes cluster with Cilium installed
- kubectl configured
- Docker (for building images)
- Helm 3+ (optional, for production deployment)

## Deployment Options

### Option 1: Docker Compose (Development)

**For local development and testing:**

```bash
# Navigate to deployment directory
cd deployments

# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

Access the application:
- **Frontend**: http://localhost:3000
- **Backend API**: http://localhost:9191
- **Redis**: localhost:6379

### Option 2: Kubernetes (Production)

#### Step 1: Build Docker Images

```bash
# Build backend image
cd web-api
docker build -t cilium-vision-api:latest .

# Build frontend image
cd ../web-ui
docker build -t cilium-vision-ui:latest .

# Tag and push to registry (replace with your registry)
docker tag cilium-vision-api:latest your-registry.com/cilium-vision-api:1.0.0
docker tag cilium-vision-ui:latest your-registry.com/cilium-vision-ui:1.0.0

docker push your-registry.com/cilium-vision-api:1.0.0
docker push your-registry.com/cilium-vision-ui:1.0.0
```

#### Step 2: Update Kubernetes Manifests

Edit `deployments/k8s/backend-deployment.yaml` and `deployments/k8s/frontend-deployment.yaml`:

```yaml
# Change image references
image: your-registry.com/cilium-vision-api:1.0.0  # backend
image: your-registry.com/cilium-vision-ui:1.0.0   # frontend
```

Edit `deployments/k8s/ingress.yaml`:

```yaml
# Update hostname
host: cilium-vision.your-domain.com
```

#### Step 3: Deploy to Kubernetes

```bash
# Create namespace (if not exists)
kubectl create namespace cilium-system

# Deploy secrets (IMPORTANT: Change JWT secret!)
kubectl apply -f deployments/k8s/secrets.yaml

# Deploy configuration
kubectl apply -f deployments/k8s/configmap.yaml

# Deploy RBAC
kubectl apply -f deployments/k8s/rbac.yaml

# Deploy Redis
kubectl apply -f deployments/k8s/redis-deployment.yaml

# Deploy backend API
kubectl apply -f deployments/k8s/backend-deployment.yaml

# Deploy frontend UI
kubectl apply -f deployments/k8s/frontend-deployment.yaml

# Deploy ingress
kubectl apply -f deployments/k8s/ingress.yaml
```

#### Step 4: Verify Deployment

```bash
# Check pod status
kubectl get pods -n cilium-system -l app=cilium-vision

# Check services
kubectl get svc -n cilium-system -l app=cilium-vision

# View logs
kubectl logs -n cilium-system -l app=cilium-vision,component=api
kubectl logs -n cilium-system -l app=cilium-vision,component=ui

# Test backend health
kubectl port-forward -n cilium-system svc/cilium-vision-api 9191:9191
curl http://localhost:9191/health

# Test frontend
kubectl port-forward -n cilium-system svc/cilium-vision-ui 3000:80
# Open browser to http://localhost:3000
```

#### Step 5: Access the Application

After deploying ingress:

```bash
# Get ingress address
kubectl get ingress -n cilium-system cilium-vision

# Access via ingress hostname
# https://cilium-vision.your-domain.com
```

### Option 3: Helm Chart (Recommended for Production)

Create a Helm chart for easier management:

```bash
# Install with Helm
helm install cilium-vision ./deployments/helm \
  --namespace cilium-system \
  --set image.backend.repository=your-registry.com/cilium-vision-api \
  --set image.backend.tag=1.0.0 \
  --set image.frontend.repository=your-registry.com/cilium-vision-ui \
  --set image.frontend.tag=1.0.0 \
  --set ingress.hostname=cilium-vision.your-domain.com

# Upgrade
helm upgrade cilium-vision ./deployments/helm -n cilium-system

# Uninstall
helm uninstall cilium-vision -n cilium-system
```

## Configuration

### Environment Variables

**Backend (API)**:
- `HOST`: Bind address (default: 0.0.0.0)
- `PORT`: Server port (default: 9191)
- `REDIS_URL`: Redis connection URL
- `JWT_SECRET`: JWT signing secret (change in production!)
- `HUBBLE_ADDRESS`: Hubble Relay address
- `K8S_CONTEXT`: Kubernetes context (optional)
- `RUST_LOG`: Log level (info, debug, trace)

**Frontend (UI)**:
- `API_URL`: Backend API URL

### Secrets Management

**IMPORTANT**: Change default secrets in production!

```bash
# Generate secure JWT secret
JWT_SECRET=$(openssl rand -base64 64)

# Create Kubernetes secret
kubectl create secret generic cilium-vision-secrets \
  --namespace cilium-system \
  --from-literal=jwt-secret="${JWT_SECRET}"
```

### TLS/HTTPS Setup

Using cert-manager for automatic certificate management:

```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create ClusterIssuer for Let's Encrypt
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: your-email@example.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF

# Ingress will automatically request certificate
```

## Scaling

### Horizontal Pod Autoscaling

```bash
# Backend API autoscaling
kubectl autoscale deployment cilium-vision-api \
  --namespace cilium-system \
  --cpu-percent=70 \
  --min=3 \
  --max=10

# Frontend UI autoscaling
kubectl autoscale deployment cilium-vision-ui \
  --namespace cilium-system \
  --cpu-percent=70 \
  --min=2 \
  --max=5
```

### Redis High Availability

For production, use Redis Sentinel or Redis Cluster:

```bash
# Install Redis HA using Bitnami Helm chart
helm repo add bitnami https://charts.bitnami.com/bitnami

helm install redis bitnami/redis \
  --namespace cilium-system \
  --set architecture=replication \
  --set auth.enabled=false \
  --set master.persistence.enabled=true \
  --set replica.replicaCount=2
```

## Monitoring

### Prometheus Metrics

The API exposes Prometheus metrics at `/metrics`:

```yaml
# ServiceMonitor for Prometheus Operator
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: cilium-vision-api
  namespace: cilium-system
spec:
  selector:
    matchLabels:
      app: cilium-vision
      component: api
  endpoints:
  - port: http
    path: /metrics
```

### Grafana Dashboards

Import pre-built dashboards from `deployments/grafana/`.

### Logging

Logs are output in JSON format for easy ingestion:

```bash
# View structured logs
kubectl logs -n cilium-system -l app=cilium-vision,component=api | jq .

# Forward to Elasticsearch
# Use Fluent Bit or Fluentd with proper filters
```

## Troubleshooting

### Common Issues

**1. Backend can't connect to Hubble**
```bash
# Check Hubble Relay is running
kubectl get pods -n kube-system -l k8s-app=hubble-relay

# Test connectivity
kubectl run test --rm -it --image=curlimages/curl -- \
  curl -v telnet://hubble-relay.kube-system.svc.cluster.local:4245
```

**2. Frontend can't reach backend**
```bash
# Check backend service
kubectl get svc -n cilium-system cilium-vision-api

# Test from within cluster
kubectl run test --rm -it --image=curlimages/curl -- \
  curl http://cilium-vision-api.cilium-system.svc.cluster.local:8080/health
```

**3. Redis connection failed**
```bash
# Check Redis
kubectl get pods -n cilium-system -l component=redis

# Test Redis
kubectl run redis-test --rm -it --image=redis:alpine -- \
  redis-cli -h cilium-vision-redis.cilium-system.svc.cluster.local ping
```

**4. Permission denied errors**
```bash
# Check RBAC
kubectl get clusterrolebinding cilium-vision-api

# Verify ServiceAccount
kubectl get sa -n cilium-system cilium-vision-api
```

### Debug Mode

Enable debug logging:

```bash
# Backend
kubectl set env deployment/cilium-vision-api \
  RUST_LOG=trace,cilium_vision_api=trace \
  -n cilium-system

# View detailed logs
kubectl logs -f -n cilium-system -l component=api
```

## Security Best Practices

1. **Change default secrets** - Never use default JWT_SECRET in production
2. **Use RBAC** - Limit ServiceAccount permissions to minimum required
3. **Enable TLS** - Always use HTTPS in production
4. **Network Policies** - Restrict pod-to-pod communication
5. **Image Scanning** - Scan container images for vulnerabilities
6. **Resource Limits** - Set appropriate CPU/memory limits
7. **Pod Security Standards** - Use restrictive security contexts

## Performance Tuning

### Backend API
```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "2Gi"
    cpu: "2000m"
```

### Redis
```yaml
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

### Frontend
```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "200m"
  limits:
    memory: "512Mi"
    cpu: "1000m"
```

## Backup & Restore

### Configuration Backup
```bash
# Backup all configurations
kubectl get all,secret,configmap,ingress -n cilium-system \
  -l app=cilium-vision -o yaml > backup.yaml

# Restore
kubectl apply -f backup.yaml
```

### Redis Data Backup
```bash
# For persistent Redis, backup PVC
kubectl get pvc -n cilium-system

# Use Velero for full backup solution
```

## Production Checklist

- [ ] Change JWT_SECRET from default
- [ ] Configure TLS/HTTPS
- [ ] Set up monitoring (Prometheus + Grafana)
- [ ] Configure logging aggregation
- [ ] Enable autoscaling
- [ ] Set resource requests/limits
- [ ] Deploy Redis HA
- [ ] Configure network policies
- [ ] Set up backups
- [ ] Test disaster recovery
- [ ] Document runbooks
- [ ] Set up alerting

---

For more information, see:
- [Architecture Documentation](WEB_APP_ARCHITECTURE.md)
- [API Documentation](API_REFERENCE.md)
- [Frontend Guide](FRONTEND_GUIDE.md)
