# CALIBER GCP Terraform Module
# Deploys CALIBER on Google Cloud Run with Cloud SQL PostgreSQL
#
# Usage:
#   module "caliber" {
#     source = "github.com/caliber-ai/caliber//terraform/modules/gcp"
#
#     project_id = "my-project"
#     region     = "us-central1"
#     name       = "caliber-prod"
#   }

terraform {
  required_version = ">= 1.5"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }
}

# -----------------------------------------------------------------------------
# LOCALS
# -----------------------------------------------------------------------------

locals {
  service_account_email = var.service_account_email != "" ? var.service_account_email : google_service_account.caliber[0].email
}

# -----------------------------------------------------------------------------
# SERVICE ACCOUNT
# -----------------------------------------------------------------------------

resource "google_service_account" "caliber" {
  count = var.service_account_email == "" ? 1 : 0

  project      = var.project_id
  account_id   = var.name
  display_name = "CALIBER Service Account"
}

resource "google_project_iam_member" "cloudsql_client" {
  count = var.service_account_email == "" ? 1 : 0

  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.caliber[0].email}"
}

resource "google_project_iam_member" "secret_accessor" {
  count = var.service_account_email == "" ? 1 : 0

  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.caliber[0].email}"
}

# -----------------------------------------------------------------------------
# CLOUD RUN SERVICE
# -----------------------------------------------------------------------------

resource "google_cloud_run_v2_service" "caliber" {
  name     = var.name
  location = var.region
  project  = var.project_id

  template {
    service_account = local.service_account_email

    scaling {
      min_instance_count = var.min_instances
      max_instance_count = var.max_instances
    }

    containers {
      image = "${var.image_repository}:${var.image_tag}"

      ports {
        container_port = 3000
      }

      resources {
        limits = {
          cpu    = var.cpu_limit
          memory = var.memory_limit
        }
        cpu_idle          = true
        startup_cpu_boost = true
      }

      env {
        name  = "RUST_LOG"
        value = "caliber_api=${var.log_level}"
      }

      env {
        name  = "CALIBER_DB_HOST"
        value = "/cloudsql/${google_sql_database_instance.caliber.connection_name}"
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
        name = "CALIBER_DB_PASSWORD"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.db_password.secret_id
            version = "latest"
          }
        }
      }

      env {
        name  = "CALIBER_DB_POOL_SIZE"
        value = tostring(var.db_pool_size)
      }

      env {
        name = "CALIBER_JWT_SECRET"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.jwt_secret.secret_id
            version = "latest"
          }
        }
      }

      env {
        name  = "CALIBER_METRICS_ENABLED"
        value = "true"
      }

      env {
        name  = "CALIBER_TRACE_SAMPLE_RATE"
        value = tostring(var.trace_sample_rate)
      }

      startup_probe {
        http_get {
          path = "/health/live"
          port = 3000
        }
        initial_delay_seconds = 10
        timeout_seconds       = 3
        period_seconds        = 5
        failure_threshold     = 30
      }

      liveness_probe {
        http_get {
          path = "/health/live"
          port = 3000
        }
        initial_delay_seconds = 0
        timeout_seconds       = 3
        period_seconds        = 30
        failure_threshold     = 3
      }

      volume_mounts {
        name       = "cloudsql"
        mount_path = "/cloudsql"
      }
    }

    volumes {
      name = "cloudsql"
      cloud_sql_instance {
        instances = [google_sql_database_instance.caliber.connection_name]
      }
    }
  }

  traffic {
    type    = "TRAFFIC_TARGET_ALLOCATION_TYPE_LATEST"
    percent = 100
  }

  labels = var.labels
}

# Allow unauthenticated access (or configure IAM for authenticated)
resource "google_cloud_run_v2_service_iam_member" "public" {
  count = var.allow_unauthenticated ? 1 : 0

  project  = var.project_id
  location = var.region
  name     = google_cloud_run_v2_service.caliber.name
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# -----------------------------------------------------------------------------
# CLOUD SQL
# -----------------------------------------------------------------------------

resource "google_sql_database_instance" "caliber" {
  name             = var.name
  project          = var.project_id
  region           = var.region
  database_version = "POSTGRES_16"

  settings {
    tier              = var.db_tier
    availability_type = var.environment == "production" ? "REGIONAL" : "ZONAL"
    disk_size         = var.db_disk_size
    disk_type         = "PD_SSD"
    disk_autoresize   = true

    backup_configuration {
      enabled                        = true
      start_time                     = "03:00"
      point_in_time_recovery_enabled = var.environment == "production"
      backup_retention_settings {
        retained_backups = var.environment == "production" ? 7 : 1
      }
    }

    ip_configuration {
      ipv4_enabled    = false
      private_network = var.vpc_network
    }

    insights_config {
      query_insights_enabled  = var.environment == "production"
      record_application_tags = true
      record_client_address   = true
    }

    maintenance_window {
      day  = 1
      hour = 4
    }

    database_flags {
      name  = "max_connections"
      value = "100"
    }

    user_labels = var.labels
  }

  deletion_protection = var.environment == "production"
}

resource "google_sql_database" "caliber" {
  name     = var.db_name
  project  = var.project_id
  instance = google_sql_database_instance.caliber.name
}

resource "google_sql_user" "caliber" {
  name     = var.db_username
  project  = var.project_id
  instance = google_sql_database_instance.caliber.name
  password = random_password.db.result
}

# -----------------------------------------------------------------------------
# SECRETS
# -----------------------------------------------------------------------------

resource "random_password" "db" {
  length  = 32
  special = false
}

resource "random_password" "jwt" {
  length  = 32
  special = false
}

resource "google_secret_manager_secret" "db_password" {
  project   = var.project_id
  secret_id = "${var.name}-db-password"

  replication {
    auto {}
  }

  labels = var.labels
}

resource "google_secret_manager_secret_version" "db_password" {
  secret      = google_secret_manager_secret.db_password.id
  secret_data = random_password.db.result
}

resource "google_secret_manager_secret" "jwt_secret" {
  project   = var.project_id
  secret_id = "${var.name}-jwt-secret"

  replication {
    auto {}
  }

  labels = var.labels
}

resource "google_secret_manager_secret_version" "jwt_secret" {
  secret      = google_secret_manager_secret.jwt_secret.id
  secret_data = random_password.jwt.result
}

# Grant service account access to secrets
resource "google_secret_manager_secret_iam_member" "db_password" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.db_password.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${local.service_account_email}"
}

resource "google_secret_manager_secret_iam_member" "jwt_secret" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.jwt_secret.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${local.service_account_email}"
}

# -----------------------------------------------------------------------------
# LOAD BALANCER (Optional - for custom domain)
# -----------------------------------------------------------------------------

resource "google_compute_region_network_endpoint_group" "caliber" {
  count = var.enable_load_balancer ? 1 : 0

  name                  = var.name
  project               = var.project_id
  region                = var.region
  network_endpoint_type = "SERVERLESS"

  cloud_run {
    service = google_cloud_run_v2_service.caliber.name
  }
}

resource "google_compute_backend_service" "caliber" {
  count = var.enable_load_balancer ? 1 : 0

  name    = var.name
  project = var.project_id

  protocol    = "HTTP"
  port_name   = "http"
  timeout_sec = 30

  backend {
    group = google_compute_region_network_endpoint_group.caliber[0].id
  }

  log_config {
    enable      = true
    sample_rate = 1.0
  }
}

resource "google_compute_url_map" "caliber" {
  count = var.enable_load_balancer ? 1 : 0

  name            = var.name
  project         = var.project_id
  default_service = google_compute_backend_service.caliber[0].id
}

resource "google_compute_target_https_proxy" "caliber" {
  count = var.enable_load_balancer ? 1 : 0

  name    = var.name
  project = var.project_id
  url_map = google_compute_url_map.caliber[0].id

  ssl_certificates = [var.ssl_certificate]
}

resource "google_compute_global_forwarding_rule" "caliber" {
  count = var.enable_load_balancer ? 1 : 0

  name       = var.name
  project    = var.project_id
  target     = google_compute_target_https_proxy.caliber[0].id
  port_range = "443"
  ip_address = var.load_balancer_ip
}
