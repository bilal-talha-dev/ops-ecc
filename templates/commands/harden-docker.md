---
description: Harden Dockerfiles, .dockerignore, docker-compose.yml, and Kubernetes manifests to pass CIS Benchmarks and `ops-ecc audit`.
---

# `/harden-docker` — Container & Compose CIS Hardening

1. Inspect all `Dockerfile*`, `docker-compose*.yml`, and Kubernetes manifest files in the repository.
2. Refactor each `Dockerfile` to:
   - Use pinned base image versions/digests (eliminate `:latest`).
   - Use multi-stage builds separating build toolchains from runtime images.
   - Run as a dedicated non-root user (`USER 10001:10001`).
   - Include a deterministic `HEALTHCHECK` instruction and strip secrets from `ENV`/`ARG`.
3. Refactor `docker-compose.yml` / Kubernetes manifests to remove `privileged: true` and host networking, enforce `read_only: true`, drop `ALL` capabilities, set `no-new-privileges:true`, and define explicit CPU/memory limits.
4. Verify with `ops-ecc audit --strict`.
