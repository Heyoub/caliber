/**
 * Pack manifest types - mirrors caliber-dsl/src/pack/schema.rs
 *
 * The pack manifest (cal.toml) is the central configuration file that defines:
 * - Providers (LLM connections)
 * - Profiles (memory retention/indexing strategies)
 * - Agents (with prompt files and toolsets)
 * - Tools (executable and prompt-based)
 * - Injections (context injection rules)
 */

export interface PackManifest {
  meta?: MetaSection;
  defaults?: DefaultsSection;
  settings?: SettingsSection;
  routing?: RoutingSection;
  profiles: Record<string, ProfileDef>;
  adapters: Record<string, AdapterDef>;
  formats: Record<string, FormatDef>;
  policies: Record<string, PolicyDef>;
  injections: Record<string, InjectionDef>;
  providers: Record<string, ProviderDef>;
  tools: ToolsSection;
  toolsets: Record<string, ToolsetDef>;
  agents: Record<string, AgentBinding>;
}

export interface MetaSection {
  version?: string;
  project?: string;
  env?: 'development' | 'staging' | 'production';
}

export interface DefaultsSection {
  context_format?: 'markdown' | 'json';
  token_budget?: number;
  strict_markdown?: boolean;
  strict_refs?: boolean;
  secrets_mode?: 'env' | 'vault' | 'inject';
}

export interface SettingsSection {
  matrix?: {
    enforce_profiles_only?: boolean;
    allowed?: Array<{
      name: string;
      retention: string;
      index: string;
      embeddings: string;
      format: string;
    }>;
  };
}

export interface RoutingSection {
  strategy?: 'first' | 'round_robin' | 'random' | 'least_latency';
  embedding_provider?: string;
  summarization_provider?: string;
}

export interface ProfileDef {
  retention: 'persistent' | 'session' | 'ephemeral';
  index: 'vector' | 'keyword' | 'hybrid' | 'none';
  embeddings: string;
  format: string;
}

export interface AdapterDef {
  type: 'postgres' | 'redis' | 'memory';
  connection: string;
  options?: Record<string, string>;
}

export interface FormatDef {
  type: string;
  include_audit?: boolean;
  include_sources?: boolean;
}

export interface PolicyDef {
  trigger: string;
  actions: PolicyActionDef[];
}

export interface PolicyActionDef {
  type: string;
  target?: string;
  max_tokens?: number;
  mode?: string;
}

export interface InjectionDef {
  entity_type?: 'note' | 'notes' | 'artifact' | 'artifacts';
  source: string;
  target: string;
  mode: 'full' | 'summary' | 'topk' | 'relevant';
  priority: number; // 0-899 (MAX_PACK_INJECTION_PRIORITY from ir.rs)
  max_tokens?: number;
  top_k?: number;
  threshold?: number;
}

export interface ProviderDef {
  type: 'openai' | 'anthropic' | 'custom';
  api_key: string;
  model: string;
  options?: Record<string, string>;
}

export interface ToolsSection {
  bin: Record<string, ToolExecDef>;
  prompts: Record<string, ToolPromptDef>;
}

export interface ToolExecDef {
  kind?: 'exec';
  cmd: string;
  timeout_ms?: number;
  allow_network?: boolean;
  allow_fs?: boolean;
  allow_subprocess?: boolean;
}

export interface ToolPromptDef {
  kind?: 'prompt';
  prompt_md: string;
  contract?: string;
  result_format?: 'json' | 'text' | 'markdown';
  timeout_ms?: number;
}

export interface ToolsetDef {
  tools: string[];
}

export interface AgentBinding {
  enabled?: boolean;
  profile: string;
  adapter?: string;
  format?: string;
  token_budget?: number;
  prompt_md: string;
  toolsets: string[];
}
