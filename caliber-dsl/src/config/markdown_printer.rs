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

    // TODO: Add other types (Memory, Policy, etc.)

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
