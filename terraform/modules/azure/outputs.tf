# CALIBER Azure Module Outputs

output "container_app_fqdn" {
  description = "Container App fully qualified domain name"
  value       = azurerm_container_app.caliber.latest_revision_fqdn
}

output "container_app_url" {
  description = "Container App URL"
  value       = "https://${azurerm_container_app.caliber.latest_revision_fqdn}"
}

output "container_app_name" {
  description = "Container App name"
  value       = azurerm_container_app.caliber.name
}

output "postgresql_fqdn" {
  description = "PostgreSQL Flexible Server FQDN"
  value       = azurerm_postgresql_flexible_server.caliber.fqdn
}

output "postgresql_id" {
  description = "PostgreSQL Flexible Server ID"
  value       = azurerm_postgresql_flexible_server.caliber.id
}

output "key_vault_id" {
  description = "Key Vault ID"
  value       = azurerm_key_vault.caliber.id
}

output "key_vault_uri" {
  description = "Key Vault URI"
  value       = azurerm_key_vault.caliber.vault_uri
}

output "managed_identity_id" {
  description = "Managed Identity ID"
  value       = azurerm_user_assigned_identity.caliber.id
}

output "managed_identity_principal_id" {
  description = "Managed Identity Principal ID"
  value       = azurerm_user_assigned_identity.caliber.principal_id
}

output "log_analytics_workspace_id" {
  description = "Log Analytics Workspace ID"
  value       = azurerm_log_analytics_workspace.caliber.id
}

output "container_app_environment_id" {
  description = "Container App Environment ID"
  value       = azurerm_container_app_environment.caliber.id
}
