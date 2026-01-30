//! Pack inspection routes.
//!
//! Provides a single endpoint to inspect the active pack/runtime configuration
//! for a tenant. This is intended as a debugging and observability surface.

use crate::db::DbClient;
use crate::error::ApiResult;
use crate::middleware::AuthExtractor;
use crate::providers::{
    EmbedRequest, EmbedResponse, PingResponse, ProviderAdapter, ProviderRegistry, SummarizeRequest,
    SummarizeResponse,
};
use crate::state::AppState;
use crate::types::PackSource;
use async_trait::async_trait;
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use caliber_core::{CaliberError, HealthStatus, LlmError, ProviderCapability, RoutingStrategy};
use caliber_dsl::compiler::{CompiledConfig as DslCompiledConfig, CompiledInjectionMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInspectInjection {
    pub source: String,
    pub target: String,
    pub entity_type: Option<String>,
    pub mode: String,
    pub priority: i32,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInspectEffectiveInjection {
    pub entity_type: String,
    pub injection: Option<PackInspectInjection>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInspectRoutingEffective {
    pub strategy: String,
    pub strategy_reason: String,
    pub embedding_provider: Option<String>,
    pub embedding_reason: String,
    pub summarization_provider: Option<String>,
    pub summarization_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInspectResponse {
    pub has_active: bool,
    pub compiled: Option<DslCompiledConfig>,
    pub pack_source: Option<PackSource>,
    pub tools: Vec<String>,
    pub toolsets: HashMap<String, Vec<String>>,
    pub agents: HashMap<String, Vec<String>>,
    pub injections: Vec<PackInspectInjection>,
    pub providers: Vec<String>,
    pub routing: Option<caliber_dsl::compiler::CompiledPackRoutingConfig>,
    pub effective_embedding_provider: Option<String>,
    pub effective_summarization_provider: Option<String>,
    pub effective_injections: Vec<PackInspectEffectiveInjection>,
    pub routing_effective: Option<PackInspectRoutingEffective>,
}

/// GET /api/v1/pack/inspect - Inspect the active pack/runtime config.
pub async fn inspect_pack(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
) -> ApiResult<impl IntoResponse> {
    let compiled = db
        .dsl_compiled_get_active(auth.tenant_id, "default")
        .await?;
    let pack_source = db
        .dsl_pack_get_active(auth.tenant_id, "default")
        .await?
        .and_then(|value| serde_json::from_value::<PackSource>(value).ok());

    let has_active = compiled.is_some();

    let (
        tools,
        toolsets,
        agents,
        injections,
        providers,
        routing,
        effective_embedding_provider,
        effective_summarization_provider,
        effective_injections,
        routing_effective,
    ) = if let Some(compiled) = compiled.as_ref() {
        let routing_effective = routing_effective(compiled).await;
        let effective_embedding_provider = routing_effective
            .as_ref()
            .and_then(|r| r.embedding_provider.clone());
        let effective_summarization_provider = routing_effective
            .as_ref()
            .and_then(|r| r.summarization_provider.clone());
        let effective_injections = effective_injections(compiled);

        (
            compiled.tools.iter().map(|t| t.id.clone()).collect(),
            compiled
                .toolsets
                .iter()
                .map(|s| (s.name.clone(), s.tools.clone()))
                .collect(),
            compiled
                .pack_agents
                .iter()
                .map(|a| (a.name.clone(), a.toolsets.clone()))
                .collect(),
            inspect_injections(compiled),
            compiled.providers.iter().map(|p| p.name.clone()).collect(),
            compiled.pack_routing.clone(),
            effective_embedding_provider,
            effective_summarization_provider,
            effective_injections,
            routing_effective,
        )
    } else {
        (
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            None,
            Vec::new(),
            None,
        )
    };

    Ok(Json(PackInspectResponse {
        has_active,
        compiled,
        pack_source,
        tools,
        toolsets,
        agents,
        injections,
        providers,
        routing,
        effective_embedding_provider,
        effective_summarization_provider,
        effective_injections,
        routing_effective,
    }))
}

async fn select_provider_name_effective(
    compiled: &DslCompiledConfig,
    capability: ProviderCapability,
) -> Option<String> {
    if compiled.providers.is_empty() {
        return None;
    }

    let providers_by_name: HashMap<&str, &caliber_dsl::compiler::CompiledProviderConfig> = compiled
        .providers
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    let preferred = compiled
        .pack_routing
        .as_ref()
        .and_then(|routing| match capability {
            ProviderCapability::Embedding => routing.embedding_provider.as_deref(),
            ProviderCapability::Summarization => routing.summarization_provider.as_deref(),
            _ => None,
        });

    if let Some(name) = preferred {
        if providers_by_name.contains_key(name) {
            return Some(name.to_string());
        }
    }

    let strategy = compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.strategy.as_deref())
        .and_then(routing_strategy_from_hint)
        .unwrap_or(RoutingStrategy::First);

    let registry = ProviderRegistry::new(strategy);
    for provider in &compiled.providers {
        let adapter: Arc<dyn ProviderAdapter> = Arc::new(PackProviderAdapter::new(&provider.name));
        registry.register(adapter).await;
    }

    registry
        .select_provider(capability)
        .await
        .ok()
        .map(|p| p.provider_id().to_string())
}

async fn routing_effective(compiled: &DslCompiledConfig) -> Option<PackInspectRoutingEffective> {
    if compiled.providers.is_empty() {
        return None;
    }

    let strategy = compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.strategy.as_deref())
        .and_then(routing_strategy_from_hint)
        .unwrap_or(RoutingStrategy::First);
    let strategy_label = routing_strategy_label(strategy);
    let strategy_reason = if compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.strategy.as_deref())
        .is_some()
    {
        "from pack routing hint".to_string()
    } else {
        "defaulted to 'first'".to_string()
    };

    let embedding_preferred = compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.embedding_provider.clone());
    let summarization_preferred = compiled
        .pack_routing
        .as_ref()
        .and_then(|r| r.summarization_provider.clone());

    let embedding_provider = if embedding_preferred.is_some() {
        embedding_preferred.clone()
    } else {
        select_provider_name_effective(compiled, ProviderCapability::Embedding).await
    };
    let embedding_reason = if embedding_preferred.is_some() {
        "from pack routing hint".to_string()
    } else {
        format!("selected via strategy '{}'", strategy_label)
    };

    let summarization_provider = if summarization_preferred.is_some() {
        summarization_preferred.clone()
    } else {
        select_provider_name_effective(compiled, ProviderCapability::Summarization).await
    };
    let summarization_reason = if summarization_preferred.is_some() {
        "from pack routing hint".to_string()
    } else {
        format!("selected via strategy '{}'", strategy_label)
    };

    Some(PackInspectRoutingEffective {
        strategy: strategy_label.to_string(),
        strategy_reason,
        embedding_provider,
        embedding_reason,
        summarization_provider,
        summarization_reason,
    })
}

fn routing_strategy_from_hint(hint: &str) -> Option<RoutingStrategy> {
    match hint.to_lowercase().as_str() {
        "first" => Some(RoutingStrategy::First),
        "round_robin" | "roundrobin" => Some(RoutingStrategy::RoundRobin),
        "random" => Some(RoutingStrategy::Random),
        "least_latency" | "leastlatency" => Some(RoutingStrategy::LeastLatency),
        _ => None,
    }
}

fn routing_strategy_label(strategy: RoutingStrategy) -> &'static str {
    match strategy {
        RoutingStrategy::First => "first",
        RoutingStrategy::RoundRobin => "round_robin",
        RoutingStrategy::Random => "random",
        RoutingStrategy::LeastLatency => "least_latency",
        RoutingStrategy::Capability(_) => "capability",
    }
}

struct PackProviderAdapter {
    id: String,
    capabilities: Vec<ProviderCapability>,
}

impl PackProviderAdapter {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            capabilities: vec![
                ProviderCapability::Embedding,
                ProviderCapability::Summarization,
            ],
        }
    }
}

#[async_trait]
impl ProviderAdapter for PackProviderAdapter {
    fn provider_id(&self) -> &str {
        &self.id
    }

    fn capabilities(&self) -> &[ProviderCapability] {
        &self.capabilities
    }

    async fn ping(&self) -> caliber_core::CaliberResult<PingResponse> {
        Ok(PingResponse {
            provider_id: self.id.clone(),
            capabilities: self.capabilities.clone(),
            latency_ms: 1,
            health: HealthStatus::Healthy,
            metadata: HashMap::new(),
        })
    }

    async fn embed(&self, _request: EmbedRequest) -> caliber_core::CaliberResult<EmbedResponse> {
        Err(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }

    async fn summarize(
        &self,
        _request: SummarizeRequest,
    ) -> caliber_core::CaliberResult<SummarizeResponse> {
        Err(CaliberError::Llm(LlmError::ProviderNotConfigured))
    }
}

fn inspect_injections(compiled: &DslCompiledConfig) -> Vec<PackInspectInjection> {
    if !compiled.pack_injections.is_empty() {
        return compiled
            .pack_injections
            .iter()
            .map(|i| PackInspectInjection {
                source: i.source.clone(),
                target: i.target.clone(),
                entity_type: i.entity_type.clone(),
                mode: mode_label(&i.mode),
                priority: i.priority,
                max_tokens: i.max_tokens,
            })
            .collect();
    }

    compiled
        .injections
        .iter()
        .map(|i| PackInspectInjection {
            source: i.source.clone(),
            target: i.target.clone(),
            entity_type: None,
            mode: mode_label(&i.mode),
            priority: i.priority,
            max_tokens: i.max_tokens,
        })
        .collect()
}

fn effective_injections(compiled: &DslCompiledConfig) -> Vec<PackInspectEffectiveInjection> {
    vec![
        effective_injection_for_entity(compiled, "note"),
        effective_injection_for_entity(compiled, "artifact"),
    ]
}

fn effective_injection_for_entity(
    compiled: &DslCompiledConfig,
    entity: &str,
) -> PackInspectEffectiveInjection {
    if !compiled.pack_injections.is_empty() {
        let mut best: Option<&caliber_dsl::compiler::CompiledPackInjectionConfig> = None;
        for injection in &compiled.pack_injections {
            let entity_match = injection.entity_type.as_deref() == Some(entity)
                || injection.entity_type.as_deref() == Some(&format!("{}s", entity));
            if !entity_match {
                continue;
            }
            match best {
                Some(current) if current.priority >= injection.priority => {}
                _ => best = Some(injection),
            }
        }

        let injection = best.map(|i| PackInspectInjection {
            source: i.source.clone(),
            target: i.target.clone(),
            entity_type: i.entity_type.clone(),
            mode: mode_label(&i.mode),
            priority: i.priority,
            max_tokens: i.max_tokens,
        });

        let reason = if injection.is_some() {
            "selected highest-priority pack injection".to_string()
        } else {
            "no pack injection targets this entity".to_string()
        };

        return PackInspectEffectiveInjection {
            entity_type: entity.to_string(),
            injection,
            reason,
        };
    }

    // Fallback to legacy DSL injection heuristics.
    let mut best: Option<&caliber_dsl::compiler::InjectionConfig> = None;
    for injection in &compiled.injections {
        let source = injection.source.to_lowercase();
        let matches_entity = match entity {
            "note" => source.contains("note"),
            "artifact" => source.contains("artifact"),
            _ => false,
        };
        if !matches_entity {
            continue;
        }
        match best {
            Some(current) if current.priority >= injection.priority => {}
            _ => best = Some(injection),
        }
    }

    let injection = best.map(|i| PackInspectInjection {
        source: i.source.clone(),
        target: i.target.clone(),
        entity_type: None,
        mode: mode_label(&i.mode),
        priority: i.priority,
        max_tokens: i.max_tokens,
    });

    let reason = if injection.is_some() {
        "selected highest-priority legacy injection (heuristic match)".to_string()
    } else {
        "no legacy injection heuristically matches this entity".to_string()
    };

    PackInspectEffectiveInjection {
        entity_type: entity.to_string(),
        injection,
        reason,
    }
}

fn mode_label(mode: &CompiledInjectionMode) -> String {
    match mode {
        CompiledInjectionMode::Full => "full".to_string(),
        CompiledInjectionMode::Summary => "summary".to_string(),
        CompiledInjectionMode::TopK { k } => format!("topk({})", k),
        CompiledInjectionMode::Relevant { threshold } => format!("relevant({})", threshold),
    }
}

/// Create the pack routes router.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/inspect", get(inspect_pack))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthContext, AuthMethod};
    use crate::db::{DbClient, DbConfig};
    use crate::middleware::AuthExtractor;
    use axum::{body::to_bytes, extract::State, response::IntoResponse};

    #[test]
    fn test_routing_strategy_from_hint() {
        assert_eq!(
            routing_strategy_from_hint("round_robin"),
            Some(RoutingStrategy::RoundRobin)
        );
        assert_eq!(
            routing_strategy_from_hint("leastlatency"),
            Some(RoutingStrategy::LeastLatency)
        );
        assert_eq!(routing_strategy_from_hint("unknown"), None);
    }

    #[test]
    fn test_routing_strategy_label() {
        assert_eq!(routing_strategy_label(RoutingStrategy::First), "first");
        assert_eq!(
            routing_strategy_label(RoutingStrategy::RoundRobin),
            "round_robin"
        );
    }

    #[tokio::test]
    async fn test_pack_provider_adapter_ping() {
        let adapter = PackProviderAdapter::new("test");
        assert_eq!(adapter.provider_id(), "test");
        assert!(adapter
            .capabilities()
            .contains(&ProviderCapability::Summarization));

        let response = adapter.ping().await.unwrap();
        assert_eq!(response.provider_id, "test");
        assert_eq!(response.health, HealthStatus::Healthy);
    }

    #[test]
    fn test_pack_inspect_response_roundtrip() {
        let response = PackInspectResponse {
            has_active: false,
            compiled: None,
            pack_source: None,
            tools: vec![],
            toolsets: HashMap::new(),
            agents: HashMap::new(),
            injections: vec![],
            providers: vec![],
            routing: None,
            effective_embedding_provider: None,
            effective_summarization_provider: None,
            effective_injections: vec![],
            routing_effective: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        let restored: PackInspectResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.has_active, restored.has_active);
        assert_eq!(response.tools, restored.tools);
    }

    struct DbTestContext {
        db: DbClient,
        auth: AuthContext,
    }

    async fn db_test_context() -> Option<DbTestContext> {
        if std::env::var("DB_TESTS").ok().as_deref() != Some("1") {
            return None;
        }

        let db = DbClient::from_config(&DbConfig::from_env()).ok()?;
        let conn = db.get_conn().await.ok()?;
        let has_fn = conn
            .query_opt(
                "SELECT 1 FROM pg_proc WHERE proname = 'caliber_tenant_create' LIMIT 1",
                &[],
            )
            .await
            .ok()
            .flatten()
            .is_some();
        if !has_fn {
            return None;
        }

        let tenant_id = db.tenant_create("test-pack", None, None).await.ok()?;
        let auth = AuthContext::new("test-user".to_string(), tenant_id, vec![], AuthMethod::Jwt);

        Some(DbTestContext { db, auth })
    }

    async fn response_json<T: serde::de::DeserializeOwned>(
        response: axum::response::Response,
    ) -> T {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("parse json")
    }

    #[tokio::test]
    async fn test_inspect_pack_db_backed_no_active() {
        let Some(ctx) = db_test_context().await else {
            return;
        };

        let response = inspect_pack(State(ctx.db.clone()), AuthExtractor(ctx.auth.clone()))
            .await
            .expect("inspect pack")
            .into_response();
        let response: PackInspectResponse = response_json(response).await;

        assert!(!response.has_active);
        assert!(response.compiled.is_none());
        assert!(response.tools.is_empty());
    }
}
