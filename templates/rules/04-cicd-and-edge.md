# Rule 04: Supply-Chain Hardened CI/CD & Edge Proxy Architecture

<!-- USER_CUSTOM_START -->
<!-- Add custom CI/CD runner tags or Cloudflare/Nginx domain policies here. -->
<!-- USER_CUSTOM_END -->

## 1. CI/CD Supply-Chain Security (GitHub Actions / GitLab CI / Azure Pipelines)
1. **Passwordless OIDC Cloud Authentication**:
   - Never store long-lived `AWS_SECRET_ACCESS_KEY` or Azure client secrets in CI/CD secrets.
   - Configure short-lived OpenID Connect (`permissions: id-token: write`) via `aws-actions/configure-aws-credentials` or `azure/login` scoped to specific repository branches/environments.
2. **Immutable Action Pinning**:
   - Pin all third-party GitHub Actions (`uses:`) to full 40-character commit SHAs with a trailing version comment (e.g., `uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2`), never mutable tags (`@main`, `@master`, `@v4`).
3. **Script Injection Prevention**:
   - Never interpolate untrusted GitHub event context (`${{ github.event.issue.title }}`, `${{ github.event.pull_request.title }}`, `${{ github.head_ref }}`) directly inside `run:` shell scripts. Pass them via intermediate `env:` variables and quote them safely in shell.
4. **Automated SARIF Security Gate**:
   - Every pipeline must execute `ops-ecc audit --format sarif --output ops-ecc.sarif --strict` and upload results to GitHub Code Scanning alongside container image scanning (`trivy`).

## 2. Edge Reverse Proxy & Cloudflare Tunnel Hardening
1. **Nginx Hardening**:
   - Disable `server_tokens off;`, enforce TLS 1.2/1.3 with AEAD cipher suites, configure HSTS (`Strict-Transport-Security`), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Content-Security-Policy`, and `limit_req_zone` rate limiting on authentication and API routes.
2. **Cloudflare Zero Trust Tunnels (`cloudflared`)**:
   - Keep origin ports closed to the public internet; expose internal services exclusively via authenticated Cloudflare Tunnel ingress rules with explicit catch-all `http_status:404`.
