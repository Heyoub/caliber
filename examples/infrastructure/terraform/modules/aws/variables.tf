# CALIBER AWS Module Variables

variable "name" {
  description = "Name prefix for all resources"
  type        = string
  default     = "caliber"
}

variable "environment" {
  description = "Environment (development, staging, production)"
  type        = string
  default     = "development"
}

variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
  default     = {}
}

# Networking
variable "vpc_id" {
  description = "VPC ID where resources will be deployed"
  type        = string
}

variable "public_subnet_ids" {
  description = "Public subnet IDs for ALB"
  type        = list(string)
}

variable "private_subnet_ids" {
  description = "Private subnet IDs for ECS tasks and RDS"
  type        = list(string)
}

# ECS
variable "image_repository" {
  description = "Container image repository"
  type        = string
  default     = "ghcr.io/caliber-ai/caliber/caliber-api"
}

variable "image_tag" {
  description = "Container image tag"
  type        = string
  default     = "latest"
}

variable "task_cpu" {
  description = "Task CPU units (1024 = 1 vCPU)"
  type        = number
  default     = 512
}

variable "task_memory" {
  description = "Task memory in MB"
  type        = number
  default     = 1024
}

variable "desired_count" {
  description = "Desired number of tasks"
  type        = number
  default     = 2
}

variable "min_capacity" {
  description = "Minimum number of tasks for auto scaling"
  type        = number
  default     = 2
}

variable "max_capacity" {
  description = "Maximum number of tasks for auto scaling"
  type        = number
  default     = 10
}

variable "enable_container_insights" {
  description = "Enable CloudWatch Container Insights"
  type        = bool
  default     = true
}

# Load Balancer
variable "certificate_arn" {
  description = "ACM certificate ARN for HTTPS"
  type        = string
}

variable "internal_lb" {
  description = "Whether the load balancer is internal"
  type        = bool
  default     = false
}

# RDS
variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "db_allocated_storage" {
  description = "Initial storage allocation in GB"
  type        = number
  default     = 20
}

variable "db_max_allocated_storage" {
  description = "Maximum storage allocation in GB for autoscaling"
  type        = number
  default     = 100
}

variable "db_name" {
  description = "Database name"
  type        = string
  default     = "caliber"
}

variable "db_username" {
  description = "Database username"
  type        = string
  default     = "caliber"
}

variable "db_pool_size" {
  description = "Database connection pool size"
  type        = number
  default     = 16
}

# Application
variable "log_level" {
  description = "Application log level"
  type        = string
  default     = "info"
}

variable "trace_sample_rate" {
  description = "OpenTelemetry trace sampling rate (0.0 - 1.0)"
  type        = number
  default     = 0.1
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 30
}
