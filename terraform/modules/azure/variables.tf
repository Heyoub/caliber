# CALIBER Azure Module Variables

variable "name" {
  description = "Name prefix for all resources"
  type        = string
  default     = "caliber"
}

variable "resource_group_name" {
  description = "Azure resource group name"
  type        = string
}

variable "location" {
  description = "Azure region"
  type        = string
  default     = "eastus"
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
variable "vnet_id" {
  description = "Virtual network ID"
  type        = string
}

variable "subnet_id" {
  description = "Subnet ID for Container Apps"
  type        = string
}

variable "db_subnet_id" {
  description = "Delegated subnet ID for PostgreSQL Flexible Server"
  type        = string
}

# Container Apps
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

variable "cpu" {
  description = "CPU cores (0.25, 0.5, 1, 2, etc.)"
  type        = number
  default     = 0.5
}

variable "memory" {
  description = "Memory (0.5Gi, 1Gi, 2Gi, etc.)"
  type        = string
  default     = "1Gi"
}

variable "min_replicas" {
  description = "Minimum number of replicas"
  type        = number
  default     = 1
}

variable "max_replicas" {
  description = "Maximum number of replicas"
  type        = number
  default     = 10
}

# PostgreSQL
variable "db_sku" {
  description = "PostgreSQL Flexible Server SKU"
  type        = string
  default     = "B_Standard_B1ms"
}

variable "db_storage_mb" {
  description = "Database storage in MB"
  type        = number
  default     = 32768 # 32 GB
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
  description = "Log Analytics workspace retention in days"
  type        = number
  default     = 30
}
