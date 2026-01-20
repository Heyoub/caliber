//! Tenant-related API types

use caliber_core::{EntityId, Timestamp};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Tenant information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TenantInfo {
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: EntityId,
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
