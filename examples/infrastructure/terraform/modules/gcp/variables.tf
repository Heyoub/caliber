# CALIBER GCP Module Variables

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

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

variable "labels" {
  description = "Labels to apply to all resources"
  type        = map(string)
  default     = {}
}

# Service Account
variable "service_account_email" {
  description = "Existing service account email (creates new if empty)"
  type        = string
  default     = ""
}

# Cloud Run
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

variable "cpu_limit" {
  description = "CPU limit (e.g., '1' or '2')"
  type        = string
  default     = "1"
}

variable "memory_limit" {
  description = "Memory limit (e.g., '512Mi' or '1Gi')"
  type        = string
  default     = "512Mi"
}

variable "min_instances" {
  description = "Minimum number of instances"
  type        = number
  default     = 0
}

variable "max_instances" {
  description = "Maximum number of instances"
  type        = number
  default     = 10
}

variable "allow_unauthenticated" {
  description = "Allow unauthenticated access to Cloud Run"
  type        = bool
  default     = true
}

# Cloud SQL
variable "vpc_network" {
  description = "VPC network self-link for private IP"
  type        = string
}

variable "db_tier" {
  description = "Cloud SQL instance tier"
  type        = string
  default     = "db-f1-micro"
}

variable "db_disk_size" {
  description = "Initial disk size in GB"
  type        = number
  default     = 10
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

# Load Balancer (Optional)
variable "enable_load_balancer" {
  description = "Enable global load balancer for custom domain"
  type        = bool
  default     = false
}

variable "ssl_certificate" {
  description = "SSL certificate self-link (required if enable_load_balancer)"
  type        = string
  default     = ""
}

variable "load_balancer_ip" {
  description = "Static IP for load balancer (required if enable_load_balancer)"
  type        = string
  default     = ""
}
