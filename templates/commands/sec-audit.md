---
description: Run a comprehensive DevSecOps security and hygiene audit using `ops-ecc audit` and remediate all Critical and High findings.
---

# `/sec-audit` — Full Repository DevSecOps Audit & Remediation

1. Execute `ops-ecc audit --format table` (and `--format sarif --output ops-ecc.sarif` if requested) across the workspace.
2. Group all findings by severity (`CRITICAL`, `HIGH`, `MEDIUM`, `LOW`) across Secrets, Dockerfiles, Compose/Kubernetes, Terraform/CloudFormation, CI/CD Workflows, and Lockfiles.
3. Provide exact, copy-pasteable code patches to remediate every `CRITICAL` and `HIGH` finding and re-run `ops-ecc audit --strict` until the scan exits with code `0`.
