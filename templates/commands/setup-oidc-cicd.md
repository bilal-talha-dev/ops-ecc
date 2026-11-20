---
description: Generate a supply-chain-hardened CI/CD pipeline with AWS/Azure OIDC federation, 40-char SHA-pinned actions, Trivy, and `ops-ecc audit --format sarif`.
---

# `/setup-oidc-cicd` — Passwordless OIDC CI/CD Pipeline Generator

1. Detect the repository's language stack, container build targets, and cloud provider (AWS or Azure).
2. Generate `.github/workflows/ci-cd-oidc.yml` (or GitLab/Azure pipeline) featuring:
   - `permissions: id-token: write, contents: read, security-events: write`.
   - All `uses:` actions pinned to immutable 40-character commit SHAs.
   - Zero script-injection interpolations in `run:` blocks.
   - Automated `ops-ecc audit --strict --format sarif --output ops-ecc.sarif` step with GitHub Code Scanning upload.
   - Multi-stage Docker build, Trivy image scan, and OIDC-authenticated deployment.
