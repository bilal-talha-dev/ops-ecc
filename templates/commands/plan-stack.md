---
description: Design a complete Full-Stack + Cloud/IaC architecture plan with mandatory App-to-IaC Infra-Sync mapping and TDD milestones.
---

# `/plan-stack` — Full-Stack + DevSecOps Architecture Planner

Execute the following workflow for the user's requested feature or system:
1. **Inspect Repository Context**: Detect existing frontend, backend, database/ORM, container, IaC (`.tf` / CloudFormation), and CI/CD files.
2. **Produce a 4-Layer Architecture Specification**:
   - **Data & Cache Layer**: PostgreSQL schema/migration diff (Prisma/Alembic) + Redis key/TTL design.
   - **API & Worker Layer**: Endpoint contracts, authn/authz guards, validation schemas, and error taxonomy.
   - **Infra-Sync Matrix**: Every required env var, secret, S3/Blob bucket, SQS/ServiceBus queue, IAM permission, and network port mapped 1-to-1 to Terraform/CloudFormation and `docker-compose.yml`.
   - **Verification & Security Gate Plan**: Unit/integration test cases + `ops-ecc audit --strict` checks.
