# CALIBER GCP Module Outputs

output "cloud_run_url" {
  description = "Cloud Run service URL"
  value       = google_cloud_run_v2_service.caliber.uri
}

output "cloud_run_name" {
  description = "Cloud Run service name"
  value       = google_cloud_run_v2_service.caliber.name
}

output "cloud_sql_connection_name" {
  description = "Cloud SQL connection name for Cloud SQL Proxy"
  value       = google_sql_database_instance.caliber.connection_name
}

output "cloud_sql_ip" {
  description = "Cloud SQL private IP address"
  value       = google_sql_database_instance.caliber.private_ip_address
}

output "service_account_email" {
  description = "Service account email"
  value       = local.service_account_email
}

output "db_password_secret_id" {
  description = "Secret Manager secret ID for database password"
  value       = google_secret_manager_secret.db_password.secret_id
}

output "jwt_secret_id" {
  description = "Secret Manager secret ID for JWT secret"
  value       = google_secret_manager_secret.jwt_secret.secret_id
}

output "load_balancer_ip" {
  description = "Load balancer IP address (if enabled)"
  value       = var.enable_load_balancer ? google_compute_global_forwarding_rule.caliber[0].ip_address : null
}
