data "aws_caller_identity" "current" {}

resource "aws_kms_key" "workload" {
  description             = "${var.project_name}-${var.environment} workload CMK"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_kms_alias" "workload" {
  name          = "alias/${var.project_name}-${var.environment}"
  target_key_id = aws_kms_key.workload.key_id
}

# 1. Modular Multi-AZ VPC & Subnets
resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true
}

resource "aws_subnet" "private_app_a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.20.1.0/24"
  availability_zone       = "${var.aws_region}a"
  map_public_ip_on_launch = false
}

resource "aws_subnet" "private_app_b" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.20.2.0/24"
  availability_zone       = "${var.aws_region}b"
  map_public_ip_on_launch = false
}

resource "aws_subnet" "isolated_db_a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.20.11.0/24"
  availability_zone       = "${var.aws_region}a"
  map_public_ip_on_launch = false
}

resource "aws_subnet" "isolated_db_b" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.20.12.0/24"
  availability_zone       = "${var.aws_region}b"
  map_public_ip_on_launch = false
}

# 2. Zero-Trust Security Groups (No 0.0.0.0/0 on SSH/RDP/DB)
resource "aws_security_group" "ecs_tasks" {
  name        = "${var.project_name}-${var.environment}-ecs-sg"
  description = "ECS Fargate application tasks security group"
  vpc_id      = aws_vpc.main.id

  ingress {
    description = "Allow HTTPS from internal VPC CIDR only"
    from_port   = 8080
    to_port     = 8080
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  egress {
    description = "Allow outbound HTTPS to AWS endpoints"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "rds_postgres" {
  name        = "${var.project_name}-${var.environment}-rds-sg"
  description = "Isolated PostgreSQL security group allowing only ECS tasks"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "PostgreSQL from ECS Fargate tasks only"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }
}

# 3. Hardened S3 Bucket with KMS Encryption & Public Access Block
resource "aws_s3_bucket" "artifacts" {
  bucket = "${var.project_name}-${var.environment}-artifacts-${data.aws_caller_identity.current.account_id}"

  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_public_access_block" "artifacts" {
  bucket                  = aws_s3_bucket.artifacts.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "artifacts" {
  bucket = aws_s3_bucket.artifacts.id
  rule {
    apply_server_side_encryption_by_default {
      kms_master_key_id = aws_kms_key.workload.arn
      sse_algorithm     = "aws:kms"
    }
  }
}

# 4. KMS-Encrypted SQS Queue + Dead-Letter Queue
resource "aws_sqs_queue" "events_dlq" {
  name              = "${var.project_name}-${var.environment}-events-dlq"
  kms_master_key_id = aws_kms_key.workload.arn
}

resource "aws_sqs_queue" "events" {
  name                       = "${var.project_name}-${var.environment}-events"
  visibility_timeout_seconds = 180
  kms_master_key_id          = aws_kms_key.workload.arn
  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.events_dlq.arn
    maxReceiveCount     = 5
  })
}

# 5. Encrypted Multi-AZ RDS PostgreSQL with prevent_destroy
resource "aws_db_subnet_group" "postgres" {
  name       = "${var.project_name}-${var.environment}-db-subnets"
  subnet_ids = [aws_subnet.isolated_db_a.id, aws_subnet.isolated_db_b.id]
}

resource "aws_db_instance" "postgres" {
  identifier                  = "${var.project_name}-${var.environment}-pg"
  engine                      = "postgres"
  engine_version              = "16.4"
  instance_class              = "db.t4g.medium"
  allocated_storage           = 50
  max_allocated_storage       = 200
  storage_encrypted           = true
  kms_key_id                  = aws_kms_key.workload.arn
  username                    = var.db_master_username
  manage_master_user_password = true
  db_subnet_group_name        = aws_db_subnet_group.postgres.name
  vpc_security_group_ids      = [aws_security_group.rds_postgres.id]
  multi_az                    = true
  publicly_accessible         = false
  backup_retention_period     = 14
  deletion_protection         = true
  skip_final_snapshot         = false
  final_snapshot_identifier   = "${var.project_name}-${var.environment}-pg-final"

  lifecycle {
    prevent_destroy = true
  }
}

# 6. ECS Fargate Cluster & Least-Privilege Task Role
resource "aws_ecs_cluster" "main" {
  name = "${var.project_name}-${var.environment}-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

resource "aws_iam_role" "ecs_task_role" {
  name = "${var.project_name}-${var.environment}-ecs-task-role"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "ecs_task_least_privilege" {
  name = "${var.project_name}-${var.environment}-ecs-policy"
  role = aws_iam_role.ecs_task_role.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid      = "ScopedS3BucketAccess"
        Effect   = "Allow"
        Action   = ["s3:GetObject", "s3:PutObject"]
        Resource = "${aws_s3_bucket.artifacts.arn}/*"
      },
      {
        Sid      = "ScopedSQSAccess"
        Effect   = "Allow"
        Action   = ["sqs:SendMessage", "sqs:ReceiveMessage", "sqs:DeleteMessage"]
        Resource = aws_sqs_queue.events.arn
      },
      {
        Sid      = "ScopedKMSDecrypt"
        Effect   = "Allow"
        Action   = ["kms:Decrypt", "kms:GenerateDataKey"]
        Resource = aws_kms_key.workload.arn
      }
    ]
  })
}
