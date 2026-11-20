---
name: docker-k8s-hardening
description: Multi-stage non-root Dockerfile templates, hardened Docker Compose resource/capability controls, and Kubernetes PodSecurity restricted profiles.
---

# Skill: Container & Kubernetes Hardening (`docker-k8s-hardening`)

## 1. Production Multi-Stage Dockerfile Blueprint
```dockerfile
FROM node:20.18.0-bookworm-slim AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts
COPY . .
RUN npm run build && npm prune --omit=dev

FROM node:20.18.0-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
RUN groupadd -g 10001 appgroup && useradd -u 10001 -g appgroup -s /sbin/nologin -M appuser
COPY --from=builder --chown=appuser:appgroup /app/dist ./dist
COPY --from=builder --chown=appuser:appgroup /app/node_modules ./node_modules
COPY --from=builder --chown=appuser:appgroup /app/package.json ./package.json
USER 10001:10001
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["node", "-e", "fetch('http://127.0.0.1:3000/healthz').then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))"]
CMD ["node", "dist/server.js"]
```

## 2. Compose & Kubernetes Security Controls
- Always set `read_only: true`, `security_opt: ["no-new-privileges:true"]`, `cap_drop: ["ALL"]`, `mem_limit: "512m"`, and `cpus: 1.0`.
- Never mount `/var/run/docker.sock` or host `/` into application containers, and never enable `privileged: true`, `network_mode: host`, or `pid: host`.
