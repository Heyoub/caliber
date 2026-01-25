//! Persistence for lightweight UI state.

use crate::nav::View;
use caliber_core::TenantId;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub active_view: View,
    pub selected_tenant_id: Option<TenantId>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub fn load(path: &Path) -> Result<Option<PersistedState>, PersistenceError> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)?;
    let state = serde_json::from_str::<PersistedState>(&contents)?;
    Ok(Some(state))
}

pub fn save(path: &Path, state: &PersistedState) -> Result<(), PersistenceError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(state)?;
    std::fs::write(path, contents)?;
    Ok(())
}
