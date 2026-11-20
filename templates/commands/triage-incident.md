---
description: Execute structured SRE incident triage, blast-radius containment, 5-Whys root-cause analysis, and safe remediation.
---

# `/triage-incident` — Production SRE Incident Triage & RCA

1. Gather symptoms: error logs, HTTP 5xx/timeout patterns, container crashloops/OOMs, database lock/connection exhaustion, or IaC drift.
2. Formulate immediate non-destructive diagnostic commands (verified against `ops-ecc hook guard-cmd`).
3. Pinpoint the root cause across application code, container limits, edge proxy timeouts, or cloud security/network rules, and implement the permanent fix with regression tests.
