# Real-World Public GitHub Repository Testing Guide

Test `ops-ecc` end-to-end on real, intentionally vulnerable public DevSecOps repositories:

## 1. Terraform & AWS/Azure IaC — [Bridgecrew TerraGoat](https://github.com/bridgecrewio/terragoat)
```bash
git clone https://github.com/bridgecrewio/terragoat.git
cd terragoat

ops-ecc doctor
ops-ecc init --yes --preset devsecops --cloud both --strict
ops-ecc audit .
ops-ecc audit . --format sarif --output terragoat.sarif

# Verify native Rust hooks (<5ms execution)
ops-ecc hook post-edit terraform/aws/ec2.tf
ops-ecc hook guard-cmd "terraform destroy -auto-approve"
```
**Expected detections:** Hardcoded AWS keys (`SEC-001`), `0.0.0.0/0` SSH/DB ingress (`IAC-001`), unencrypted RDS/Neptune (`IAC-002`), public S3/RDS (`IAC-003`), wildcard IAM (`IAC-004`), missing `prevent_destroy` (`IAC-005`), missing S3 state lock (`IAC-006`), mutable GitHub Actions (`CICD-001`), and Dockerfile CIS (`DOCKER-002`/`003`).

---

## 2. AWS CloudFormation — [Bridgecrew CfnGoat](https://github.com/bridgecrewio/cfngoat)
```bash
git clone https://github.com/bridgecrewio/cfngoat.git
cd cfngoat

ops-ecc init --yes --preset devops --cloud aws --strict
ops-ecc audit . --severity medium

# Compare against ops-ecc's hardened CloudFormation scaffold (passes with 0 findings)
ops-ecc scaffold cfn-aws --out ./hardened-cfn
ops-ecc audit ./hardened-cfn --strict
```

---

## 3. Kubernetes & Docker CIS — [Kubernetes Goat](https://github.com/madhuakula/kubernetes-goat)
```bash
git clone https://github.com/madhuakula/kubernetes-goat.git
cd kubernetes-goat

ops-ecc audit .
```
**Expected detections:** `privileged: true` (`K8S-001`), `hostNetwork`/`hostPID` (`K8S-002`), `/var/run/docker.sock` & `/` host mounts (`K8S-003`), missing `resources.limits` (`K8S-005`), `ENV`/`ARG` secrets (`DOCKER-004`), `:latest` tags (`DOCKER-001`), `curl | bash` (`DOCKER-005`), and missing `USER`/`HEALTHCHECK` (`DOCKER-002`/`003`).
