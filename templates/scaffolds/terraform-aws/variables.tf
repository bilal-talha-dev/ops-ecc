variable "aws_region" {
  type        = string
  description = "Primary AWS region for deployment"
  default     = "us-east-1"
}

variable "project_name" {
  type        = string
  description = "Project identifier prefix"
  default     = "ops-ecc-app"
}

variable "environment" {
  type        = string
  description = "Deployment environment (dev, staging, prod)"
  default     = "prod"
}

variable "vpc_cidr" {
  type        = string
  description = "VPC IPv4 CIDR block"
  default     = "10.20.0.0/16"
}

variable "db_master_username" {
  type        = string
  description = "RDS PostgreSQL master username"
  default     = "app_db_admin"
}
