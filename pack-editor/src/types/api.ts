/**
 * API response types for pack operations
 *
 * These types mirror the Rust API responses from caliber-api.
 * Backend endpoints will be added later - these are stubs for now.
 */

export type DslConfigStatus = 'Draft' | 'Deployed' | 'Archived';

// --- Pack History ---

export interface PackVersionSummary {
  config_id: string;
  name: string;
  version: number;
  status: DslConfigStatus;
  created_at: string;
  deployed_at: string | null;
  deployed_by: string | null;
  file_count: number;
  notes: string | null;
}

export interface PackHistoryResponse {
  versions: PackVersionSummary[];
  active_version: number | null;
}

// --- Pack Content ---

export interface PackSourceFile {
  path: string;
  content: string;
}

export interface PackVersionResponse {
  config_id: string;
  name: string;
  version: number;
  status: DslConfigStatus;
  manifest: string;
  files: PackSourceFile[];
  compiled: object | null;
  created_at: string;
}

// --- Pack Diff ---

export interface PackDiffResponse {
  from_version: number;
  to_version: number;
  manifest_diff: TextDiff | null;
  file_diffs: FileDiff[];
  added_files: string[];
  removed_files: string[];
}

export interface TextDiff {
  hunks: DiffHunk[];
}

export interface FileDiff {
  path: string;
  diff_type: 'Modified' | 'Added' | 'Removed';
  hunks: DiffHunk[];
}

export interface DiffHunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  lines: DiffLine[];
}

export interface DiffLine {
  type: 'context' | 'add' | 'remove';
  content: string;
}

// --- Compose/Deploy ---

export interface ComposePackResponse {
  success: boolean;
  ast: object | null;
  compiled: object | null;
  dsl_source: string | null;
  errors: PackDiagnostic[];
}

export interface PackDiagnostic {
  file: string;
  line: number;
  column: number;
  message: string;
}

export interface DeployDslRequest {
  name: string;
  source: string;
  pack?: {
    manifest: string;
    markdowns: PackSourceFile[];
  };
  activate: boolean;
  notes?: string;
}

export interface DeployDslResponse {
  config_id: string;
  name: string;
  version: number;
  status: DslConfigStatus;
  message: string;
}

// --- Revert ---

export interface PackRevertRequest {
  source_config_id: string;
  name: string;
  activate: boolean;
  notes?: string;
}

// --- Inspect (existing endpoint) ---

export interface PackInspectResponse {
  has_active: boolean;
  compiled: object | null;
  pack_source: {
    manifest: string;
    markdowns: PackSourceFile[];
  } | null;
  tools: string[];
  toolsets: Record<string, string[]>;
  agents: Record<string, string[]>;
  providers: string[];
}
