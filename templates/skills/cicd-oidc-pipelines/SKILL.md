---
name: cicd-oidc-pipelines
description: Supply-chain-hardened CI/CD workflows using AWS/Azure OIDC federation, 40-char commit SHA pinning, safe environment variable handling, and SARIF code scanning.
---

# Skill: Supply-Chain Hardened OIDC CI/CD (`cicd-oidc-pipelines`)

## 1. GitHub Actions Security Checklist
1. **Top-Level Minimal Permissions**:
   ```yaml
   permissions:
     contents: read
     id-token: write
     security-events: write
   ```
2. **40-Character Commit SHA Pinning**:
   Always pin `uses:` to a full 40-char commit SHA (`actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683`).
3. **Eliminate Script Injection**:
   Move untrusted `${{ github.event.* }}` expressions into step `env:` variables and reference `"$PR_TITLE"` safely in bash.
