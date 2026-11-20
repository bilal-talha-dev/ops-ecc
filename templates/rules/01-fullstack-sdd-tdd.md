# Rule 01: Full-Stack SDD/TDD & Mandatory Infra-Sync Governance

<!-- USER_CUSTOM_START -->
<!-- Add team-specific full-stack overrides inside this block. Preserved across `ops-ecc update`. -->
<!-- USER_CUSTOM_END -->

## 1. Specification-Driven & Test-Driven Development (SDD + TDD)
1. **Contract-First Engineering**: Before implementing any API endpoint, background worker, or database migration across Next.js/React, Node/Express, FastAPI, Django, or Rust services:
   - Define the strict schema contract (OpenAPI 3.1 / Zod / Pydantic v2 / SQLite-or-Postgres Prisma/Alembic migration).
   - Write failing unit and integration tests covering happy path, validation boundaries, authz/tenant isolation, and idempotency (`RED` -> `GREEN` -> `REFACTOR`).
2. **Database & Cache Hygiene**:
   - All PostgreSQL schema changes MUST be version-controlled via Prisma (`prisma/migrations/`) or Alembic (`alembic/versions/`).
   - Every foreign key and high-cardinality filter column must have an explicit index; wrap multi-step writes in ACID transactions.
   - Redis keys must follow namespaced TTL conventions (`<service>:<env>:<entity>:<id>`) with explicit eviction/fallback handling.

## 2. Mandatory Infra-Sync Rule (App <-> IaC Coupling)
Whenever application code introduces, renames, or removes any external infrastructure dependency, you MUST update the corresponding Infrastructure-as-Code (Terraform `.tf`, CloudFormation `template.yaml`, and `docker-compose.yml`) in the **same atomic change**:
1. **Environment Variables & Secrets**:
   - Adding `process.env.X`, `os.environ["X"]`, or `std::env::var("X")` requires Simultaneous declaration in `.env.example`, container task definitions (ECS Fargate / Kubernetes Deployment / Azure Container Apps), and SSM Parameter Store / Secrets Manager / Key Vault references.
2. **Storage Buckets, Queues & Databases**:
   - Referencing an S3 bucket, Azure Blob container, SQS queue, Service Bus topic, RDS instance, or Redis cluster in application code requires matching IaC resource blocks, least-privilege IAM/RBAC policies, and encryption-at-rest configurations.
3. **Network & Port Bindings**:
   - Changing an application listener port or healthcheck endpoint (`/healthz`, `/readyz`) requires synchronized updates in `Dockerfile` (`EXPOSE` + `HEALTHCHECK`), target group health checks, Kubernetes probes, and Nginx/Cloudflare ingress routes.
