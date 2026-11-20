# Rule 02: Production Infrastructure-as-Code (Terraform & AWS CloudFormation)

<!-- USER_CUSTOM_START -->
<!-- Add custom organization Terraform/CloudFormation tagging standards here. -->
<!-- USER_CUSTOM_END -->

## 1. Terraform (`.tf`) Engineering Standards
1. **State & Backend Locking**:
   - Every root Terraform module MUST configure an encrypted remote backend (`s3` with `encrypt = true` and `dynamodb_table` state lock or `use_lockfile = true`, or `azurerm` with Blob lease locking).
   - Pin `required_version` and all provider versions in `versions.tf` using pessimistic constraints (`~>`).
2. **Modular Architecture & Naming**:
   - Segregate environments (`envs/dev`, `envs/staging`, `envs/prod`) from reusable modules (`modules/vpc`, `modules/ecs`, `modules/rds`).
   - Every variable in `variables.tf` must declare explicit `type`, `description`, and `validation` blocks where applicable; mark secrets with `sensitive = true`.
3. **Stateful Resource Protection**:
   - Production databases (`aws_db_instance`, `aws_rds_cluster`, `azurerm_mssql_database`), S3 buckets with audit/state data, and KMS keys MUST include `lifecycle { prevent_destroy = true }` and automated backup retention (`backup_retention_period >= 7`).

## 2. AWS CloudFormation (`template.yaml` / `.json`) Standards
1. **Deletion & Update Policies**:
   - All stateful resources (`AWS::RDS::DBInstance`, `AWS::RDS::DBCluster`, `AWS::DynamoDB::Table`, `AWS::S3::Bucket`, `AWS::KMS::Key`) MUST specify `DeletionPolicy: Retain` and `UpdateReplacePolicy: Retain`.
2. **Parameter Constraints & NoEcho**:
   - Sensitive parameters MUST set `NoEcho: true` or resolve dynamically via `{{resolve:secretsmanager:...}}` / `{{resolve:ssm-secure:...}}`.
3. **Validation Gate**:
   - Every edit to `.tf` or CloudFormation `.yaml`/`.json` files triggers `ops-ecc hook post-edit <file>` to enforce syntax validity, encryption defaults, and zero `0.0.0.0/0` administrative ingress.
