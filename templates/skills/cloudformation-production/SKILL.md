---
name: cloudformation-production
description: Enterprise AWS CloudFormation & SAM authoring patterns with DeletionPolicy Retain, KMS encryption, NoEcho secrets, and least-privilege IAM roles.
---

# Skill: Enterprise AWS CloudFormation (`cloudformation-production`)

## 1. Stateful Safeguards
Every stateful CloudFormation resource MUST declare both `DeletionPolicy: Retain` and `UpdateReplacePolicy: Retain`:
```yaml
Resources:
  PrimaryDatabase:
    Type: AWS::RDS::DBInstance
    DeletionPolicy: Retain
    UpdateReplacePolicy: Retain
    Properties:
      Engine: postgres
      StorageEncrypted: true
      KmsKeyId: !GetAtt DatabaseKmsKey.Arn
      PubliclyAccessible: false
      DeletionProtection: true
      BackupRetentionPeriod: 14
```

## 2. Security Group & IAM Guardrails
- Never allow `CidrIp: 0.0.0.0/0` or `CidrIpv6: ::/0` on SSH (`22`), RDP (`3389`), or database ports (`5432`, `3306`, `1433`, `27017`, `6379`).
- Never use `Action: "*"` or `Resource: "*"` in `AWS::IAM::Role` or `AWS::IAM::Policy`; scope to resource ARNs using `!Sub` or `!GetAtt`.
