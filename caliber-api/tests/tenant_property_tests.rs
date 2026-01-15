//! Property-Based Tests for Tenant Isolation
//!
//! **Property 5: Tenant Isolation**
//!
//! For any authenticated request with a tenant context header, the API SHALL
//! return ONLY data belonging to that tenant, AND mutations SHALL only affect
//! that tenant's data.
//!
//! **Validates: Requirements 1.6, 2.5**

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{get, post},
    Router,
};
use caliber_api::{
    auth::{generate_jwt_token, AuthConfig},
    db::{DbClient, DbConfig},
    middleware::{auth_middleware, AuthMiddlewareState},
    types::{CreateTrajectoryRequest, CreateScopeRequest},
};
use caliber_core::EntityId;
use proptest::prelude::*;
use tower::ServiceExt;
use uuid::Uuid;

// ====================