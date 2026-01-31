/**
 * Zod validation schemas for pack manifest (cal.toml)
 *
 * These schemas mirror the validation rules from caliber-dsl:
 * - caliber-dsl/src/pack/schema.rs (structure)
 * - caliber-dsl/src/pack/ir.rs (constraints like MAX_PACK_INJECTION_PRIORITY)
 */

import { z } from 'zod';

// Constants from caliber-dsl/src/pack/ir.rs
export const MAX_PACK_INJECTION_PRIORITY = 899;

// Forbidden shell characters for tool commands
const FORBIDDEN_SHELL_CHARS = /[;|&$`(){}><!\n\r]/;
const PATH_TRAVERSAL = /\.\./;

// --- Section schemas ---

export const metaSectionSchema = z
  .object({
    version: z.string().optional(),
    project: z.string().optional(),
    env: z.enum(['development', 'staging', 'production']).optional(),
  })
  .strict();

export const defaultsSectionSchema = z
  .object({
    context_format: z.enum(['markdown', 'json']).optional(),
    token_budget: z.number().int().positive().optional(),
    strict_markdown: z.boolean().optional(),
    strict_refs: z.boolean().optional(),
    secrets_mode: z.enum(['env', 'vault', 'inject']).optional(),
  })
  .strict();

export const routingSectionSchema = z
  .object({
    strategy: z
      .enum(['first', 'round_robin', 'random', 'least_latency'])
      .optional(),
    embedding_provider: z.string().optional(),
    summarization_provider: z.string().optional(),
  })
  .strict();

export const profileDefSchema = z
  .object({
    retention: z.enum(['persistent', 'session', 'ephemeral']),
    index: z.enum(['vector', 'keyword', 'hybrid', 'none']),
    embeddings: z.string(),
    format: z.string(),
  })
  .strict();

export const adapterDefSchema = z
  .object({
    type: z.enum(['postgres', 'redis', 'memory']),
    connection: z.string(),
    options: z.record(z.string()).optional(),
  })
  .strict();

export const formatDefSchema = z
  .object({
    type: z.string(),
    include_audit: z.boolean().optional(),
    include_sources: z.boolean().optional(),
  })
  .strict();

export const policyActionDefSchema = z
  .object({
    type: z.string(),
    target: z.string().optional(),
    max_tokens: z.number().int().positive().optional(),
    mode: z.string().optional(),
  })
  .strict();

export const policyDefSchema = z
  .object({
    trigger: z.string(),
    actions: z.array(policyActionDefSchema),
  })
  .strict();

export const injectionDefSchema = z
  .object({
    entity_type: z
      .enum(['note', 'notes', 'artifact', 'artifacts'])
      .optional(),
    source: z.string(),
    target: z.string(),
    mode: z.enum(['full', 'summary', 'topk', 'relevant']),
    priority: z.number().int().min(0).max(MAX_PACK_INJECTION_PRIORITY),
    max_tokens: z.number().int().positive().optional(),
    top_k: z.number().int().positive().optional(),
    threshold: z.number().min(0).max(1).optional(),
  })
  .strict()
  .refine(
    (data) => {
      if (data.mode === 'topk' && data.top_k === undefined) return false;
      if (data.mode === 'relevant' && data.threshold === undefined)
        return false;
      return true;
    },
    { message: 'topk mode requires top_k, relevant mode requires threshold' }
  );

export const providerDefSchema = z
  .object({
    type: z.enum(['openai', 'anthropic', 'custom']),
    api_key: z.string(),
    model: z.string(),
    options: z.record(z.string()).optional(),
  })
  .strict();

// Tool command validation (from caliber-dsl/src/pack/ir.rs)
export const toolExecDefSchema = z
  .object({
    kind: z.literal('exec').optional(),
    cmd: z
      .string()
      .refine((cmd) => cmd.startsWith('./') || cmd.startsWith('/'), {
        message: 'cmd must start with ./ or /',
      })
      .refine((cmd) => !FORBIDDEN_SHELL_CHARS.test(cmd), {
        message: 'cmd contains forbidden shell characters: ; | & $ ` ( ) { } < > ! or newlines',
      })
      .refine((cmd) => !PATH_TRAVERSAL.test(cmd), {
        message: 'cmd contains path traversal (..)',
      }),
    timeout_ms: z.number().int().positive().optional(),
    allow_network: z.boolean().optional(),
    allow_fs: z.boolean().optional(),
    allow_subprocess: z.boolean().optional(),
  })
  .strict();

export const toolPromptDefSchema = z
  .object({
    kind: z.literal('prompt').optional(),
    prompt_md: z.string(),
    contract: z.string().optional(),
    result_format: z.enum(['json', 'text', 'markdown']).optional(),
    timeout_ms: z.number().int().positive().optional(),
  })
  .strict();

export const toolsSectionSchema = z
  .object({
    bin: z.record(toolExecDefSchema).default({}),
    prompts: z.record(toolPromptDefSchema).default({}),
  })
  .strict();

export const toolsetDefSchema = z
  .object({
    tools: z.array(z.string()),
  })
  .strict();

export const agentBindingSchema = z
  .object({
    enabled: z.boolean().optional(),
    profile: z.string(),
    adapter: z.string().optional(),
    format: z.string().optional(),
    token_budget: z.number().int().positive().optional(),
    prompt_md: z.string(),
    toolsets: z.array(z.string()).default([]),
  })
  .strict();

export const settingsSectionSchema = z
  .object({
    matrix: z
      .object({
        enforce_profiles_only: z.boolean().optional(),
        allowed: z
          .array(
            z.object({
              name: z.string(),
              retention: z.string(),
              index: z.string(),
              embeddings: z.string(),
              format: z.string(),
            })
          )
          .optional(),
      })
      .optional(),
  })
  .strict();

// --- Full manifest schema ---

export const packManifestSchema = z
  .object({
    meta: metaSectionSchema.optional(),
    defaults: defaultsSectionSchema.optional(),
    settings: settingsSectionSchema.optional(),
    routing: routingSectionSchema.optional(),
    profiles: z.record(profileDefSchema).default({}),
    adapters: z.record(adapterDefSchema).default({}),
    formats: z.record(formatDefSchema).default({}),
    policies: z.record(policyDefSchema).default({}),
    injections: z.record(injectionDefSchema).default({}),
    providers: z.record(providerDefSchema).default({}),
    tools: toolsSectionSchema.default({ bin: {}, prompts: {} }),
    toolsets: z.record(toolsetDefSchema).default({}),
    agents: z.record(agentBindingSchema).default({}),
  })
  .strict();

export type PackManifestZod = z.infer<typeof packManifestSchema>;

// --- Validation helpers ---

/**
 * Convert Zod errors to PackDiagnostic format
 */
export function zodErrorsToDiagnostics(
  error: z.ZodError,
  file: string = 'cal.toml'
): import('../types').PackDiagnostic[] {
  return error.issues.map((issue) => ({
    file,
    line: 0, // TOML parser would provide this
    column: 0,
    message: `${issue.path.join('.')}: ${issue.message}`,
  }));
}
