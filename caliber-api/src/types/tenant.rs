//! Tenant-related API types

use caliber_core::{TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Request to create a new tenant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Email domain for auto-association (e.g., "acme.com")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Additional tenant settings
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub settings: Option<serde_json::Value>,
}

/// Request to update an existing tenant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpdateTenantRequest {
    /// Updated tenant name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Updated email domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Updated tenant status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TenantStatus>,
    /// Updated tenant settings
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub settings: Option<serde_json::Value>,
}

/// Tenant information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TenantInfo {
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    /// Tenant name
    pub name: String,
    /// Email domain for auto-association (e.g., "acme.com")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// WorkOS organization ID for enterprise SSO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workos_organization_id: Option<String>,
    /// Tenant status
    pub status: TenantStatus,
    /// When the tenant was created
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: Timestamp,
}

/// Tenant status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TenantStatus {
    /// Tenant is active and operational
    Active,
    /// Tenant is suspended
    Suspended,
    /// Tenant is archived
    Archived,
}

impl fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TenantStatus::Active => "Active",
            TenantStatus::Suspended => "Suspended",
            TenantStatus::Archived => "Archived",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for TenantStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "active" => Ok(TenantStatus::Active),
            "suspended" => Ok(TenantStatus::Suspended),
            "archived" => Ok(TenantStatus::Archived),
            _ => Err(format!("Invalid TenantStatus: {}", s)),
        }
    }
}

/// Response containing a list of tenants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListTenantsResponse {
    /// List of tenants
    pub tenants: Vec<TenantInfo>,
}

// ============================================================================
// CHANGE JOURNAL TYPES (Cache Invalidation)
// ============================================================================

/// Operation type for change journal entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ChangeOperation {
    /// Entity was inserted
    Insert,
    /// Entity was updated
    Update,
    /// Entity was deleted
    Delete,
}

impl fmt::Display for ChangeOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ChangeOperation::Insert => "INSERT",
            ChangeOperation::Update => "UPDATE",
            ChangeOperation::Delete => "DELETE",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for ChangeOperation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "INSERT" => Ok(ChangeOperation::Insert),
            "UPDATE" => Ok(ChangeOperation::Update),
            "DELETE" => Ok(ChangeOperation::Delete),
            _ => Err(format!("Invalid ChangeOperation: {}", s)),
        }
    }
}

/// A record from the change journal, representing a single entity change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChangeRecord {
    /// Monotonically increasing change ID (watermark)
    pub change_id: i64,
    /// Tenant that owns the changed entity
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    /// Type of entity that changed (e.g., "trajectory", "scope", "artifact")
    pub entity_type: String,
    /// ID of the changed entity (can be any entity type)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub entity_id: uuid::Uuid,
    /// Type of operation performed
    pub operation: ChangeOperation,
    /// When the change occurred
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub changed_at: Timestamp,
}

/// Response for watermark query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WatermarkResponse {
    /// Current watermark (highest change_id for the tenant)
    pub watermark: i64,
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
}

/// Response for changes query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChangesResponse {
    /// List of change records
    pub changes: Vec<ChangeRecord>,
    /// New watermark (highest change_id in results, or input watermark if no changes)
    pub watermark: i64,
    /// Whether there are more changes beyond the limit
    pub has_more: bool,
}
