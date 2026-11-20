---
name: nginx-cloudflare-edge
description: Hardened Nginx reverse proxy configurations (TLS 1.3, HSTS, CSP, rate limiting) paired with Cloudflare Zero Trust Tunnel (`cloudflared`) origin isolation.
---

# Skill: Hardened Nginx & Cloudflare Zero Trust Edge (`nginx-cloudflare-edge`)

## 1. Nginx Edge Security Directives
- Set `server_tokens off;`, `client_max_body_size 10m;`, and configure `limit_req_zone $binary_remote_addr zone=api_limit:10m rate=20r/s;`.
- Add strict response headers:
  - `Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;`
  - `X-Content-Type-Options "nosniff" always;`
  - `X-Frame-Options "DENY" always;`
  - `Referrer-Policy "strict-origin-when-cross-origin" always;`

## 2. Cloudflare Tunnel (`config.yml`) Origin Protection
- Run `cloudflared` as a non-root service connecting outbound-only to Cloudflare Edge so zero public inbound ports (`80`/`443`) need to be opened on the origin VPC security group.
- Always terminate `ingress:` rules with `- service: http_status:404`.
