# ─── Stage 1: Build React App ────────────────────────────────
FROM node:22-alpine AS builder
WORKDIR /ui
COPY package*.json ./
RUN npm ci --silent
COPY . ./
RUN npm run build

# ─── Stage 2: Serve with Nginx ───────────────────────────────
FROM nginx:alpine

RUN addgroup -g 1000 cilium && adduser -D -u 1000 -G cilium cilium && \
    mkdir -p /var/cache/nginx /var/run /var/log/nginx && \
    chown -R cilium:cilium /var/cache/nginx /var/run /var/log/nginx /usr/share/nginx/html

COPY --from=builder --chown=cilium:cilium /ui/dist /usr/share/nginx/html

# Nginx config with API proxy and WebSocket support
COPY <<'EOF' /etc/nginx/conf.d/default.conf
server {
    listen 8080;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;

    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml;
    gzip_min_length 256;

    location /api/ {
        proxy_pass http://cilium-vision-api:9191;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_connect_timeout 5s;
        proxy_read_timeout 60s;
    }

    location /ws/ {
        proxy_pass http://cilium-vision-api:9191;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
        proxy_buffering off;
    }

    location /health { proxy_pass http://cilium-vision-api:9191; }
    location /ready  { proxy_pass http://cilium-vision-api:9191; }
    location /metrics { proxy_pass http://cilium-vision-api:9191; }

    location / {
        try_files $uri $uri/ /index.html;
    }

    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
        expires 30d;
        add_header Cache-Control "public, immutable";
    }
}
EOF

USER cilium
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:8080/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
