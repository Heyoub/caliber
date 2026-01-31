//! Canonical Markdown generator for config AST
//! Generates deterministic Markdown output for round-trip testing

use crate::parser::ast::*;

/// Convert AST to canonical Markdown format
pub fn ast_to_markdown(ast: &CaliberAst) -> String {
    let mut output = String::new();

    // Sort definitions by type and name for deterministic output
    let mut adapters = Vec::new();
    let mut memories = Vec::new();
    let mut policies = Vec::new();
    let mut injections = Vec::new();
    let mut providers = Vec::new();
    let mut caches = Vec::new();
    let mut trajectories = Vec::new();
    let mut agents = Vec::new();

    for def in &ast.definitions {
        match def {
            Definition::Adapter(a) => adapters.push(a),
            Definition::Memory(m) => memories.push(m),
            Definition::Policy(p) => policies.push(p),
            Definition::Injection(i) => injections.push(i),
            Definition::Provider(prov) => providers.push(prov),
            Definition::Cache(c) => caches.push(c),
            Definition::Trajectory(t) => trajectories.push(t),
            Definition::Agent(a) => agents.push(a),
            _ => {}, // Evolution, SummarizationPolicy not implemented yet
        }
    }

    // Sort alphabetically by name
    adapters.sort_by(|a, b| a.name.cmp(&b.name));
    memories.sort_by(|a, b| a.name.cmp(&b.name));
    policies.sort_by(|a, b| a.name.cmp(&b.name));
    providers.sort_by(|a, b| a.name.cmp(&b.name));
    trajectories.sort_by(|a, b| a.name.cmp(&b.name));
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    caches.sort_by(|a, b| a.backend.cmp(&b.backend)); // Cache has no name, sort by backend
    // Injections have no name field - sort by (source, target) tuple
    injections.sort_by(|a, b| {
        a.source.cmp(&b.source)
            .then_with(|| a.target.cmp(&b.target))
    });

    // Generate Markdown for each type
    for adapter in adapters {
        output.push_str(&format!("```adapter {}\n", adapter.name));
        output.push_str(&format!("adapter_type: {}\n", adapter_type_to_string(&adapter.adapter_type)));
        output.push_str(&format!("connection: {}\n", adapter.connection));
        if !adapter.options.is_empty() {
            output.push_str("options:\n");
            for (k, v) in &adapter.options {
                output.push_str(&format!("  {}: {}\n", k, v));
            }
        }
        output.push_str("```\n\n");
    }

    for provider in providers {
        output.push_str(&format!("```provider {}\n", provider.name));
        output.push_str(&format!("provider_type: {}\n", provider_type_to_string(&provider.provider_type)));
        output.push_str(&format!("api_key: {}\n", env_value_to_string(&provider.api_key)));
        output.push_str(&format!("model: {}\n", provider.model));
        if !provider.options.is_empty() {
            output.push_str("options:\n");
            for (k, v) in &provider.options {
                output.push_str(&format!("  {}: {}\n", k, v));
            }
        }
        output.push_str("```\n\n");
    }

    for memory in memories {
        output.push_str(&format!("```memory {}\n", memory.name));
        output.push_str(&format!("memory_type: {}\n", memory_type_to_string(&memory.memory_type)));
        output.push_str("schema:\n");
        for field in &memory.schema {
            output.push_str(&format!("  - name: {}\n", field.name));
            output.push_str(&format!("    type: {}\n", field_type_to_string(&field.field_type)));
            output.push_str(&format!("    nullable: {}\n", field.nullable));
            if let Some(default) = &field.default {
                output.push_str(&format!("    default: {}\n", default));
            }
        }
        output.push_str(&format!("retention: {}\n", retention_to_string(&memory.retention)));
        output.push_str(&format!("lifecycle: {}\n", lifecycle_to_string(&memory.lifecycle)));
        if let Some(parent) = &memory.parent {
            output.push_str(&format!("parent: {}\n", parent));
        }
        if !memory.indexes.is_empty() {
            output.push_str("indexes:\n");
            for index in &memory.indexes {
                output.push_str(&format!("  - field: {}\n", index.field));
                output.push_str(&format!("    type: {}\n", index_type_to_string(&index.index_type)));
            }
        }
        if !memory.inject_on.is_empty() {
            output.push_str("inject_on:\n");
            for trigger in &memory.inject_on {
                output.push_str(&format!("  - {}\n", trigger_to_string(trigger)));
            }
        }
        if !memory.artifacts.is_empty() {
            output.push_str("artifacts:\n");
            for artifact in &memory.artifacts {
                output.push_str(&format!("  - {}\n", artifact));
            }
        }
        if !memory.modifiers.is_empty() {
            output.push_str("modifiers:\n");
            for modifier in &memory.modifiers {
                output.push_str(&format!("  - {}\n", modifier_to_string(modifier)));
            }
        }
        output.push_str("```\n\n");
    }

    for policy in policies {
        output.push_str(&format!("```policy {}\n", policy.name));
        output.push_str("rules:\n");
        for rule in &policy.rules {
            output.push_str(&format!("  - trigger: {}\n", trigger_to_string(&rule.trigger)));
            output.push_str("    actions:\n");
            for action in &rule.actions {
                output.push_str(&format!("      - {}\n", action_to_string(action)));
            }
        }
        output.push_str("```\n\n");
    }

    for injection in injections {
        output.push_str("```injection\n");
        output.push_str(&format!("source: {}\n", injection.source));
        output.push_str(&format!("target: {}\n", injection.target));
        output.push_str(&format!("mode: {}\n", injection_mode_to_string(&injection.mode)));
        output.push_str(&format!("priority: {}\n", injection.priority));
        if let Some(max_tokens) = injection.max_tokens {
            output.push_str(&format!("max_tokens: {}\n", max_tokens));
        }
        if let Some(filter) = &injection.filter {
            output.push_str(&format!("filter: {}\n", filter_expr_to_string(filter)));
        }
        output.push_str("```\n\n");
    }

    for cache in caches {
        output.push_str("```cache\n");
        output.push_str(&format!("backend: {}\n", cache_backend_to_string(&cache.backend)));
        if let Some(path) = &cache.path {
            output.push_str(&format!("path: {}\n", path));
        }
        output.push_str(&format!("size_mb: {}\n", cache.size_mb));
        output.push_str(&format!("default_freshness: {}\n", freshness_to_string(&cache.default_freshness)));
        if let Some(max_entries) = cache.max_entries {
            output.push_str(&format!("max_entries: {}\n", max_entries));
        }
        if let Some(ttl) = &cache.ttl {
            output.push_str(&format!("ttl: {}\n", ttl));
        }
        output.push_str("```\n\n");
    }

    for trajectory in trajectories {
        output.push_str(&format!("```trajectory {}\n", trajectory.name));
        if let Some(description) = &trajectory.description {
            output.push_str(&format!("description: {}\n", description));
        }
        output.push_str(&format!("agent_type: {}\n", trajectory.agent_type));
        output.push_str(&format!("token_budget: {}\n", trajectory.token_budget));
        output.push_str("memory_refs:\n");
        for mem_ref in &trajectory.memory_refs {
            output.push_str(&format!("  - {}\n", mem_ref));
        }
        if let Some(metadata) = &trajectory.metadata {
            output.push_str(&format!("metadata: {}\n", metadata));
        }
        output.push_str("```\n\n");
    }

    for agent in agents {
        output.push_str(&format!("```agent {}\n", agent.name));
        output.push_str("capabilities:\n");
        for capability in &agent.capabilities {
            output.push_str(&format!("  - {}\n", capability));
        }
        output.push_str("constraints:\n");
        output.push_str(&format!("  max_concurrent: {}\n", agent.constraints.max_concurrent));
        output.push_str(&format!("  timeout_ms: {}\n", agent.constraints.timeout_ms));
        output.push_str("permissions:\n");
        output.push_str("  read:\n");
        for r in &agent.permissions.read {
            output.push_str(&format!("    - {}\n", r));
        }
        output.push_str("  write:\n");
        for w in &agent.permissions.write {
            output.push_str(&format!("    - {}\n", w));
        }
        output.push_str("  lock:\n");
        for l in &agent.permissions.lock {
            output.push_str(&format!("    - {}\n", l));
        }
        output.push_str("```\n\n");
    }

    output
}

fn adapter_type_to_string(t: &AdapterType) -> &'static str {
    match t {
        AdapterType::Postgres => "postgres",
        AdapterType::Redis => "redis",
        AdapterType::Memory => "memory",
    }
}

fn provider_type_to_string(t: &ProviderType) -> &'static str {
    match t {
        ProviderType::OpenAI => "openai",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Custom => "custom",
    }
}

fn env_value_to_string(v: &EnvValue) -> String {
    match v {
        EnvValue::Env(var) => format!("env:{}", var),
        EnvValue::Literal(s) => s.clone(),
    }
}

fn memory_type_to_string(t: &MemoryType) -> &'static str {
    match t {
        MemoryType::Ephemeral => "ephemeral",
        MemoryType::Working => "working",
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
        MemoryType::Meta => "meta",
    }
}

fn field_type_to_string(t: &FieldType) -> String {
    match t {
        FieldType::Uuid => "uuid".to_string(),
        FieldType::Text => "text".to_string(),
        FieldType::Int => "int".to_string(),
        FieldType::Float => "float".to_string(),
        FieldType::Bool => "bool".to_string(),
        FieldType::Timestamp => "timestamp".to_string(),
        FieldType::Json => "json".to_string(),
        FieldType::Embedding(dim) => {
            if let Some(d) = dim {
                format!("embedding({})", d)
            } else {
                "embedding".to_string()
            }
        }
        FieldType::Enum(variants) => format!("enum({})", variants.join(", ")),
        FieldType::Array(inner) => format!("array({})", field_type_to_string(inner)),
    }
}

fn retention_to_string(r: &Retention) -> String {
    match r {
        Retention::Persistent => "persistent".to_string(),
        Retention::Session => "session".to_string(),
        Retention::Scope => "scope".to_string(),
        Retention::Duration(d) => format!("duration({})", d),
        Retention::Max(n) => format!("max({})", n),
    }
}

fn lifecycle_to_string(l: &Lifecycle) -> String {
    match l {
        Lifecycle::Explicit => "explicit".to_string(),
        Lifecycle::AutoClose(trigger) => format!("auto_close({})", trigger_to_string(trigger)),
    }
}

fn trigger_to_string(t: &Trigger) -> String {
    match t {
        Trigger::TaskStart => "task_start".to_string(),
        Trigger::TaskEnd => "task_end".to_string(),
        Trigger::ScopeClose => "scope_close".to_string(),
        Trigger::TurnEnd => "turn_end".to_string(),
        Trigger::Manual => "manual".to_string(),
        Trigger::Schedule(s) => format!("schedule:{}", s),
    }
}

fn index_type_to_string(t: &IndexType) -> &'static str {
    match t {
        IndexType::Btree => "btree",
        IndexType::Hash => "hash",
        IndexType::Gin => "gin",
        IndexType::Hnsw => "hnsw",
        IndexType::Ivfflat => "ivfflat",
    }
}

fn modifier_to_string(m: &ModifierDef) -> String {
    match m {
        ModifierDef::Embeddable { provider } => format!("embeddable(provider: {})", provider),
        ModifierDef::Summarizable { style, on_triggers } => {
            let style_str = match style {
                SummaryStyle::Brief => "brief",
                SummaryStyle::Detailed => "detailed",
            };
            let triggers_str = on_triggers.iter()
                .map(trigger_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("summarizable(style: {}, on: [{}])", style_str, triggers_str)
        }
        ModifierDef::Lockable { mode } => {
            let mode_str = match mode {
                LockMode::Exclusive => "exclusive",
                LockMode::Shared => "shared",
            };
            format!("lockable(mode: {})", mode_str)
        }
    }
}

fn action_to_string(a: &Action) -> String {
    match a {
        Action::Summarize(target) => format!("summarize({})", target),
        Action::ExtractArtifacts(target) => format!("extract_artifacts({})", target),
        Action::Checkpoint(target) => format!("checkpoint({})", target),
        Action::Prune { target, criteria } => {
            format!("prune(target: {}, criteria: {})", target, filter_expr_to_string(criteria))
        }
        Action::Notify(msg) => format!("notify({})", msg),
        Action::Inject { target, mode } => {
            format!("inject(target: {}, mode: {})", target, injection_mode_to_string(mode))
        }
        Action::AutoSummarize { source_level, target_level, create_edges } => {
            format!("auto_summarize(source: {:?}, target: {:?}, edges: {})", source_level, target_level, create_edges)
        }
    }
}

fn injection_mode_to_string(m: &InjectionMode) -> String {
    match m {
        InjectionMode::Full => "full".to_string(),
        InjectionMode::Summary => "summary".to_string(),
        InjectionMode::TopK(k) => format!("topk({})", k),
        InjectionMode::Relevant(threshold) => format!("relevant({})", threshold),
    }
}

fn filter_expr_to_string(f: &FilterExpr) -> String {
    match f {
        FilterExpr::Comparison { field, op, value } => {
            let op_str = match op {
                CompareOp::Eq => "==",
                CompareOp::Ne => "!=",
                CompareOp::Gt => ">",
                CompareOp::Lt => "<",
                CompareOp::Ge => ">=",
                CompareOp::Le => "<=",
                CompareOp::Contains => "contains",
                CompareOp::Regex => "regex",
                CompareOp::In => "in",
            };
            let value_str = match value {
                FilterValue::String(s) => format!("\"{}\"", s),
                FilterValue::Number(n) => n.to_string(),
                FilterValue::Bool(b) => b.to_string(),
                FilterValue::Null => "null".to_string(),
                FilterValue::CurrentTrajectory => "current_trajectory".to_string(),
                FilterValue::CurrentScope => "current_scope".to_string(),
                FilterValue::Now => "now".to_string(),
                FilterValue::Array(_) => "[...]".to_string(), // Simplified
            };
            format!("{} {} {}", field, op_str, value_str)
        }
        FilterExpr::And(exprs) => {
            let exprs_str = exprs.iter()
                .map(filter_expr_to_string)
                .collect::<Vec<_>>()
                .join(" AND ");
            format!("({})", exprs_str)
        }
        FilterExpr::Or(exprs) => {
            let exprs_str = exprs.iter()
                .map(filter_expr_to_string)
                .collect::<Vec<_>>()
                .join(" OR ");
            format!("({})", exprs_str)
        }
        FilterExpr::Not(expr) => format!("NOT {}", filter_expr_to_string(expr)),
    }
}

fn cache_backend_to_string(b: &CacheBackendType) -> &'static str {
    match b {
        CacheBackendType::Lmdb => "lmdb",
        CacheBackendType::Memory => "memory",
    }
}

fn freshness_to_string(f: &FreshnessDef) -> String {
    match f {
        FreshnessDef::BestEffort { max_staleness } => format!("best_effort(max_staleness: {})", max_staleness),
        FreshnessDef::Strict => "strict".to_string(),
    }
}
