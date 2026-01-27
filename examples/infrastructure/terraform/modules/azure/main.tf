# CALIBER Azure Terraform Module
# Deploys CALIBER on Azure Container Instances with Flexible Server PostgreSQL
#
# Usage:
#   module "caliber" {
#     source = "github.com/caliber-ai/caliber//terraform/modules/azure"
#
#     name                = "caliber-prod"
#     resource_group_name = "caliber-rg"
#     location            = "eastus"
#     subnet_id           = "/subscriptions/.../subnets/containers"
#     db_subnet_id        = "/subscriptions/.../subnets/database"
#   }

terraform {
  required_version = ">= 1.5"
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}

# -----------------------------------------------------------------------------
# DATA SOURCES
# -----------------------------------------------------------------------------

data "azurerm_resource_group" "caliber" {
  name = var.resource_group_name
}

data "azurerm_client_config" "current" {}

# -----------------------------------------------------------------------------
# KEY VAULT
# -----------------------------------------------------------------------------

resource "azurerm_key_vault" "caliber" {
  name                = "${var.name}-kv"
  location            = var.location
  resource_group_name = var.resource_group_name
  tenant_id           = data.azurerm_client_config.current.tenant_id
  sku_name            = "standard"

  soft_delete_retention_days = 7
  purge_protection_enabled   = var.environment == "production"

  access_policy {
    tenant_id = data.azurerm_client_config.current.tenant_id
    object_id = data.azurerm_client_config.current.object_id

    secret_permissions = [
      "Get",
      "List",
      "Set",
      "Delete",
      "Purge",
    ]
  }

  tags = var.tags
}

resource "random_password" "db" {
  length  = 32
  special = false
}

resource "random_password" "jwt" {
  length  = 32
  special = false
}

resource "azurerm_key_vault_secret" "db_password" {
  name         = "db-password"
  value        = random_password.db.result
  key_vault_id = azurerm_key_vault.caliber.id
}

resource "azurerm_key_vault_secret" "jwt_secret" {
  name         = "jwt-secret"
  value        = random_password.jwt.result
  key_vault_id = azurerm_key_vault.caliber.id
}

# -----------------------------------------------------------------------------
# POSTGRESQL FLEXIBLE SERVER
# -----------------------------------------------------------------------------

resource "azurerm_private_dns_zone" "postgres" {
  name                = "${var.name}.postgres.database.azure.com"
  resource_group_name = var.resource_group_name

  tags = var.tags
}

resource "azurerm_private_dns_zone_virtual_network_link" "postgres" {
  name                  = "${var.name}-postgres"
  private_dns_zone_name = azurerm_private_dns_zone.postgres.name
  virtual_network_id    = var.vnet_id
  resource_group_name   = var.resource_group_name

  tags = var.tags
}

resource "azurerm_postgresql_flexible_server" "caliber" {
  name                = var.name
  resource_group_name = var.resource_group_name
  location            = var.location

  version                      = "16"
  sku_name                     = var.db_sku
  storage_mb                   = var.db_storage_mb
  backup_retention_days        = var.environment == "production" ? 7 : 1
  geo_redundant_backup_enabled = var.environment == "production"

  delegated_subnet_id = var.db_subnet_id
  private_dns_zone_id = azurerm_private_dns_zone.postgres.id

  administrator_login    = var.db_username
  administrator_password = random_password.db.result

  zone = "1"

  high_availability {
    mode                      = var.environment == "production" ? "ZoneRedundant" : "Disabled"
    standby_availability_zone = var.environment == "production" ? "2" : null
  }

  maintenance_window {
    day_of_week  = 1
    start_hour   = 4
    start_minute = 0
  }

  tags = var.tags

  depends_on = [azurerm_private_dns_zone_virtual_network_link.postgres]
}

resource "azurerm_postgresql_flexible_server_database" "caliber" {
  name      = var.db_name
  server_id = azurerm_postgresql_flexible_server.caliber.id
  charset   = "UTF8"
  collation = "en_US.utf8"
}

# -----------------------------------------------------------------------------
# CONTAINER APPS ENVIRONMENT
# -----------------------------------------------------------------------------

resource "azurerm_log_analytics_workspace" "caliber" {
  name                = var.name
  location            = var.location
  resource_group_name = var.resource_group_name
  sku                 = "PerGB2018"
  retention_in_days   = var.log_retention_days

  tags = var.tags
}

resource "azurerm_container_app_environment" "caliber" {
  name                       = var.name
  location                   = var.location
  resource_group_name        = var.resource_group_name
  log_analytics_workspace_id = azurerm_log_analytics_workspace.caliber.id

  infrastructure_subnet_id = var.subnet_id

  tags = var.tags
}

# -----------------------------------------------------------------------------
# CONTAINER APP
# -----------------------------------------------------------------------------

resource "azurerm_user_assigned_identity" "caliber" {
  name                = var.name
  resource_group_name = var.resource_group_name
  location            = var.location

  tags = var.tags
}

resource "azurerm_key_vault_access_policy" "container" {
  key_vault_id = azurerm_key_vault.caliber.id
  tenant_id    = data.azurerm_client_config.current.tenant_id
  object_id    = azurerm_user_assigned_identity.caliber.principal_id

  secret_permissions = [
    "Get",
    "List",
  ]
}

resource "azurerm_container_app" "caliber" {
  name                         = var.name
  container_app_environment_id = azurerm_container_app_environment.caliber.id
  resource_group_name          = var.resource_group_name
  revision_mode                = "Single"

  identity {
    type         = "UserAssigned"
    identity_ids = [azurerm_user_assigned_identity.caliber.id]
  }

  template {
    min_replicas = var.min_replicas
    max_replicas = var.max_replicas

    container {
      name   = "caliber-api"
      image  = "${var.image_repository}:${var.image_tag}"
      cpu    = var.cpu
      memory = var.memory

      env {
        name  = "RUST_LOG"
        value = "caliber_api=${var.log_level}"
      }

      env {
        name  = "CALIBER_DB_HOST"
        value = azurerm_postgresql_flexible_server.caliber.fqdn
      }

      env {
        name  = "CALIBER_DB_PORT"
        value = "5432"
      }

      env {
        name  = "CALIBER_DB_NAME"
        value = var.db_name
      }

      env {
        name  = "CALIBER_DB_USER"
        value = var.db_username
      }

      env {
        name        = "CALIBER_DB_PASSWORD"
        secret_name = "db-password"
      }

      env {
        name  = "CALIBER_DB_POOL_SIZE"
        value = tostring(var.db_pool_size)
      }

      env {
        name        = "CALIBER_JWT_SECRET"
        secret_name = "jwt-secret"
      }

      env {
        name  = "CALIBER_METRICS_ENABLED"
        value = "true"
      }

      env {
        name  = "CALIBER_TRACE_SAMPLE_RATE"
        value = tostring(var.trace_sample_rate)
      }

      liveness_probe {
        transport = "HTTP"
        path      = "/health/live"
        port      = 3000
      }

      readiness_probe {
        transport = "HTTP"
        path      = "/health/ready"
        port      = 3000
      }

      startup_probe {
        transport        = "HTTP"
        path             = "/health/live"
        port             = 3000
        failure_count_threshold = 30
      }
    }
  }

  ingress {
    external_enabled = true
    target_port      = 3000

    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }

  secret {
    name                = "db-password"
    key_vault_secret_id = azurerm_key_vault_secret.db_password.id
    identity            = azurerm_user_assigned_identity.caliber.id
  }

  secret {
    name                = "jwt-secret"
    key_vault_secret_id = azurerm_key_vault_secret.jwt_secret.id
    identity            = azurerm_user_assigned_identity.caliber.id
  }

  tags = var.tags

  depends_on = [azurerm_key_vault_access_policy.container]
}

# -----------------------------------------------------------------------------
# AUTOSCALING
# -----------------------------------------------------------------------------

# Container Apps has built-in autoscaling via template.min_replicas/max_replicas
# For HTTP-based scaling, add scale rules:

# resource "azurerm_container_app" "caliber" {
#   ...
#   template {
#     ...
#     http_scale_rule {
#       name                = "http-requests"
#       concurrent_requests = 100
#     }
#   }
# }
