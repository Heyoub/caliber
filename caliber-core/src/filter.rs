//! Unified filter expressions for search and DSL
//!
//! This module provides a unified FilterExpr type that can be used across
//! both the API layer (for search queries) and the DSL layer (for policy filters).

use serde::{Deserialize, Serialize};

/// Filter operator for field comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum FilterOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Less than
    Lt,
    /// Greater than or equal
    Gte,
    /// Less than or equal
    Lte,
    /// Contains substring (for strings)
    Contains,
    /// In list of values
    In,
    /// Matches regular expression
    Regex,
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical NOT
    Not,
}

/// Unified filter expression for queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FilterExpr {
    /// Field to filter on
    pub field: String,
    /// Operator to apply
    pub operator: FilterOperator,
    /// Value to compare against (JSON value for flexibility)
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub value: serde_json::Value,
}

impl FilterExpr {
    /// Create a new filter expression.
    pub fn new(
        field: impl Into<String>,
        operator: FilterOperator,
        value: serde_json::Value,
    ) -> Self {
        Self {
            field: field.into(),
            operator,
            value,
        }
    }

    /// Create an equality filter.
    pub fn eq(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self::new(field, FilterOperator::Eq, value)
    }

    /// Create a contains filter.
    pub fn contains(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self::new(field, FilterOperator::Contains, value)
    }
}
