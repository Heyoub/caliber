/**
 * Pinia store for pack editor state management
 *
 * Handles:
 * - Pack content (manifest + files)
 * - Validation pipeline (parse → schema → refs → server → ready)
 * - Version history
 * - Dirty state tracking
 * - Dependency graph computation
 */

import { defineStore } from 'pinia';
import { parse as parseTOML } from '@smol-toml/toml';
import { packManifestSchema, zodErrorsToDiagnostics } from '../schemas/manifest.zod';
import { packApi } from '../services/api/pack';
import type {
  PackManifest,
  PackSourceFile,
  PackDiagnostic,
  PackVersionSummary,
  DslConfigStatus,
  EditorFile,
  FileTreeNode,
  DependencyEdge,
  ValidationStage,
  FileLanguage,
} from '../types';

// Template for new packs
const TEMPLATE_MANIFEST = `[meta]
version = "1.0"
project = "my-pack"
env = "development"

[defaults]
context_format = "markdown"
token_budget = 8000

[providers.openai]
type = "openai"
api_key = "env:OPENAI_API_KEY"
model = "text-embedding-3-small"

[profiles.default]
retention = "persistent"
index = "vector"
embeddings = "openai"
format = "markdown"

[toolsets.core]
tools = []

[agents.main]
enabled = true
profile = "default"
token_budget = 8000
prompt_md = "agents/main.agent.md"
toolsets = ["core"]
`;

export interface PackStoreState {
  // Pack identity
  packName: string;
  currentVersion: number | null;
  configId: string | null;
  status: DslConfigStatus;

  // Content
  manifestSource: string;
  manifest: PackManifest | null;
  files: EditorFile[];
  activeFilePath: string | null;

  // Validation
  validationStage: ValidationStage;
  diagnostics: PackDiagnostic[];

  // History
  versions: PackVersionSummary[];
  activeVersion: number | null;

  // UI state
  isDirty: boolean;
  isLoading: boolean;
  viewMode: 'raw' | 'structured';
}

export const usePackStore = defineStore('pack', {
  state: (): PackStoreState => ({
    packName: '',
    currentVersion: null,
    configId: null,
    status: 'Draft',

    manifestSource: '',
    manifest: null,
    files: [],
    activeFilePath: null,

    validationStage: 'idle',
    diagnostics: [],

    versions: [],
    activeVersion: null,

    isDirty: false,
    isLoading: false,
    viewMode: 'raw',
  }),

  getters: {
    // --- Manifest accessors ---
    agents: (state) => state.manifest?.agents ?? {},
    providers: (state) => state.manifest?.providers ?? {},
    profiles: (state) => state.manifest?.profiles ?? {},
    toolsets: (state) => state.manifest?.toolsets ?? {},
    injections: (state) => state.manifest?.injections ?? {},
    adapters: (state) => state.manifest?.adapters ?? {},

    tools: (state) => state.manifest?.tools ?? { bin: {}, prompts: {} },

    /**
     * All tool IDs in the manifest
     */
    allToolIds(): string[] {
      const t = this.tools;
      return [
        ...Object.keys(t.bin).map((k) => `tools.bin.${k}`),
        ...Object.keys(t.prompts).map((k) => `tools.prompts.${k}`),
      ];
    },

    // --- Active file ---
    activeFile: (state) =>
      state.files.find((f) => f.path === state.activeFilePath),

    // --- Validation status ---
    isValid: (state) =>
      state.validationStage === 'ready' && state.diagnostics.length === 0,
    hasErrors: (state) => state.diagnostics.length > 0,

    // --- File tree structure ---
    fileTree(): FileTreeNode[] {
      return buildFileTree(this.files);
    },

    // --- Dependency graph ---
    dependencyGraph(): DependencyEdge[] {
      if (!this.manifest) return [];
      return buildDependencyGraph(this.manifest, this.files);
    },
  },

  actions: {
    // ==========================================================================
    // LOADING
    // ==========================================================================

    /**
     * Load a pack by name and optionally a specific version
     */
    async loadPack(name: string, version?: number) {
      this.isLoading = true;
      try {
        // Load history
        const history = await packApi.getHistory(name);
        this.versions = history.versions;
        this.activeVersion = history.active_version;

        // Determine version to load
        const targetVersion =
          version ?? history.active_version ?? history.versions[0]?.version;
        if (!targetVersion) throw new Error('No versions found');

        const versionInfo = history.versions.find(
          (v) => v.version === targetVersion
        );
        if (!versionInfo) throw new Error(`Version ${targetVersion} not found`);

        // Load full version content
        const pack = await packApi.getVersion(versionInfo.config_id);

        this.packName = pack.name;
        this.currentVersion = pack.version;
        this.configId = pack.config_id;
        this.status = pack.status;
        this.manifestSource = pack.manifest;
        this.files = [
          createEditorFile('cal.toml', pack.manifest, 'toml'),
          ...pack.files.map((f) =>
            createEditorFile(f.path, f.content, getFileLanguage(f.path))
          ),
        ];
        this.activeFilePath = 'cal.toml';
        this.isDirty = false;

        await this.revalidate();
      } finally {
        this.isLoading = false;
      }
    },

    /**
     * Create a new empty pack
     */
    async createNewPack(name: string) {
      this.packName = name;
      this.currentVersion = null;
      this.configId = null;
      this.status = 'Draft';
      this.manifestSource = TEMPLATE_MANIFEST;
      this.files = [createEditorFile('cal.toml', TEMPLATE_MANIFEST, 'toml')];
      this.activeFilePath = 'cal.toml';
      this.isDirty = true;
      this.versions = [];
      this.activeVersion = null;

      await this.revalidate();
    },

    /**
     * Load from local files (for development/testing)
     */
    loadFromLocal(manifest: string, files: PackSourceFile[]) {
      this.packName = 'local';
      this.currentVersion = null;
      this.configId = null;
      this.status = 'Draft';
      this.manifestSource = manifest;
      this.files = [
        createEditorFile('cal.toml', manifest, 'toml'),
        ...files.map((f) =>
          createEditorFile(f.path, f.content, getFileLanguage(f.path))
        ),
      ];
      this.activeFilePath = 'cal.toml';
      this.isDirty = false;

      this.revalidate();
    },

    // ==========================================================================
    // EDITING
    // ==========================================================================

    /**
     * Update a file's content
     */
    updateFile(path: string, content: string) {
      const file = this.files.find((f) => f.path === path);
      if (file) {
        file.content = content;
        file.isDirty = content !== file.originalContent;
        this.isDirty = this.files.some((f) => f.isDirty);

        if (path === 'cal.toml') {
          this.manifestSource = content;
        }

        this.revalidate();
      }
    },

    /**
     * Add a new file
     */
    addFile(path: string, content: string = '') {
      const language = getFileLanguage(path);
      this.files.push(createEditorFile(path, content, language, true));
      this.isDirty = true;
      this.activeFilePath = path;
    },

    /**
     * Delete a file
     */
    deleteFile(path: string) {
      if (path === 'cal.toml') return; // Cannot delete manifest
      this.files = this.files.filter((f) => f.path !== path);
      this.isDirty = true;
      if (this.activeFilePath === path) {
        this.activeFilePath = 'cal.toml';
      }
      this.revalidate();
    },

    /**
     * Set the active file for editing
     */
    setActiveFile(path: string) {
      this.activeFilePath = path;
    },

    /**
     * Toggle between raw and structured view mode
     */
    setViewMode(mode: 'raw' | 'structured') {
      this.viewMode = mode;
    },

    // ==========================================================================
    // VALIDATION
    // ==========================================================================

    /**
     * Run the validation pipeline
     */
    async revalidate() {
      this.diagnostics = [];

      // Stage 1: Parse TOML
      this.validationStage = 'parse';
      try {
        this.manifest = parseTOML(this.manifestSource) as PackManifest;
      } catch (e: unknown) {
        const err = e as { line?: number; column?: number; message: string };
        this.diagnostics.push({
          file: 'cal.toml',
          line: err.line ?? 1,
          column: err.column ?? 0,
          message: err.message,
        });
        return;
      }

      // Stage 2: Schema validation (Zod)
      this.validationStage = 'schema';
      const schemaResult = packManifestSchema.safeParse(this.manifest);
      if (!schemaResult.success) {
        this.diagnostics = zodErrorsToDiagnostics(schemaResult.error);
        return;
      }

      // Stage 3: Reference validation
      this.validationStage = 'refs';
      const refErrors = this.validateReferences();
      if (refErrors.length > 0) {
        this.diagnostics = refErrors;
        return;
      }

      // Stage 4: Server-side validation is optional and debounced
      // Could call packApi.compose() here for full validation

      this.validationStage = 'ready';
    },

    /**
     * Validate cross-references in the manifest
     */
    validateReferences(): PackDiagnostic[] {
      const errors: PackDiagnostic[] = [];
      const manifest = this.manifest!;
      const filePaths = new Set(this.files.map((f) => f.path));

      const profiles = new Set(Object.keys(manifest.profiles ?? {}));
      const adapters = new Set(Object.keys(manifest.adapters ?? {}));
      const toolsets = new Set(Object.keys(manifest.toolsets ?? {}));
      const providers = new Set(Object.keys(manifest.providers ?? {}));
      const formats = new Set(Object.keys(manifest.formats ?? {}));

      // Validate agents
      for (const [name, agent] of Object.entries(manifest.agents ?? {})) {
        if (!profiles.has(agent.profile)) {
          errors.push({
            file: 'cal.toml',
            line: 0,
            column: 0,
            message: `agents.${name}.profile: unknown profile '${agent.profile}'`,
          });
        }

        if (agent.adapter && !adapters.has(agent.adapter)) {
          errors.push({
            file: 'cal.toml',
            line: 0,
            column: 0,
            message: `agents.${name}.adapter: unknown adapter '${agent.adapter}'`,
          });
        }

        if (agent.format && !formats.has(agent.format)) {
          errors.push({
            file: 'cal.toml',
            line: 0,
            column: 0,
            message: `agents.${name}.format: unknown format '${agent.format}'`,
          });
        }

        for (const ts of agent.toolsets ?? []) {
          if (!toolsets.has(ts)) {
            errors.push({
              file: 'cal.toml',
              line: 0,
              column: 0,
              message: `agents.${name}.toolsets: unknown toolset '${ts}'`,
            });
          }
        }

        // Check prompt file exists
        const promptPath = agent.prompt_md;
        const promptExists =
          filePaths.has(promptPath) ||
          Array.from(filePaths).some((p) => p.endsWith(promptPath));
        if (!promptExists) {
          errors.push({
            file: 'cal.toml',
            line: 0,
            column: 0,
            message: `agents.${name}.prompt_md: file '${promptPath}' not found`,
          });
        }
      }

      // Validate toolsets reference valid tools
      const toolIds = new Set(this.allToolIds);
      for (const [name, toolset] of Object.entries(manifest.toolsets ?? {})) {
        for (const tool of toolset.tools) {
          if (!toolIds.has(tool)) {
            errors.push({
              file: 'cal.toml',
              line: 0,
              column: 0,
              message: `toolsets.${name}.tools: unknown tool '${tool}'`,
            });
          }
        }
      }

      // Validate routing providers
      if (
        manifest.routing?.embedding_provider &&
        !providers.has(manifest.routing.embedding_provider)
      ) {
        errors.push({
          file: 'cal.toml',
          line: 0,
          column: 0,
          message: `routing.embedding_provider: unknown provider '${manifest.routing.embedding_provider}'`,
        });
      }

      if (
        manifest.routing?.summarization_provider &&
        !providers.has(manifest.routing.summarization_provider)
      ) {
        errors.push({
          file: 'cal.toml',
          line: 0,
          column: 0,
          message: `routing.summarization_provider: unknown provider '${manifest.routing.summarization_provider}'`,
        });
      }

      // Validate profile embeddings reference providers
      for (const [name, profile] of Object.entries(manifest.profiles ?? {})) {
        if (!providers.has(profile.embeddings)) {
          errors.push({
            file: 'cal.toml',
            line: 0,
            column: 0,
            message: `profiles.${name}.embeddings: unknown provider '${profile.embeddings}'`,
          });
        }
      }

      return errors;
    },

    // ==========================================================================
    // SAVE / DEPLOY
    // ==========================================================================

    /**
     * Save the pack (optionally deploy/activate)
     */
    async save(activate: boolean = false) {
      const packSource = {
        manifest: this.manifestSource,
        markdowns: this.files
          .filter((f) => f.path !== 'cal.toml')
          .map((f) => ({ path: f.path, content: f.content })),
      };

      const response = await packApi.deploy({
        name: this.packName,
        source: '',
        pack: packSource,
        activate,
        notes: activate ? 'Deployed from Pack Editor' : undefined,
      });

      // Update state
      this.configId = response.config_id;
      this.currentVersion = response.version;
      this.status = response.status;
      this.isDirty = false;
      this.files.forEach((f) => {
        f.isDirty = false;
        f.originalContent = f.content;
      });

      // Reload history
      const history = await packApi.getHistory(this.packName);
      this.versions = history.versions;
      this.activeVersion = history.active_version;

      return response;
    },

    /**
     * Revert to a previous version (creates new version from old)
     */
    async revertTo(configId: string) {
      const response = await packApi.revert({
        source_config_id: configId,
        name: this.packName,
        activate: false,
      });

      // Reload the reverted version
      await this.loadPack(this.packName, response.version);

      return response;
    },

    /**
     * Compare two versions
     */
    async compareVersions(fromConfigId: string, toConfigId: string) {
      return await packApi.diff(fromConfigId, toConfigId);
    },
  },
});

// =============================================================================
// HELPERS
// =============================================================================

function createEditorFile(
  path: string,
  content: string,
  language: FileLanguage,
  isNew: boolean = false
): EditorFile {
  return {
    path,
    content,
    originalContent: isNew ? '' : content,
    isDirty: isNew,
    language,
  };
}

function getFileLanguage(path: string): FileLanguage {
  if (path.endsWith('.toml')) return 'toml';
  if (path.endsWith('.json')) return 'json';
  if (path.endsWith('.yaml') || path.endsWith('.yml')) return 'yaml';
  return 'markdown';
}

function buildFileTree(files: EditorFile[]): FileTreeNode[] {
  const root: FileTreeNode[] = [];
  const folders = new Map<string, FileTreeNode>();

  // Sort files so folders come first
  const sortedFiles = [...files].sort((a, b) => a.path.localeCompare(b.path));

  for (const file of sortedFiles) {
    const parts = file.path.split('/');
    const fileName = parts.pop()!;

    let parent = root;
    let currentPath = '';

    // Create folder nodes
    for (const part of parts) {
      currentPath = currentPath ? `${currentPath}/${part}` : part;

      if (!folders.has(currentPath)) {
        const folderNode: FileTreeNode = {
          name: part,
          path: currentPath,
          type: 'folder',
          children: [],
        };
        folders.set(currentPath, folderNode);
        parent.push(folderNode);
      }

      parent = folders.get(currentPath)!.children!;
    }

    // Add file node
    parent.push({
      name: fileName,
      path: file.path,
      type: 'file',
      language: file.language,
      isDirty: file.isDirty,
    });
  }

  return root;
}

function buildDependencyGraph(
  manifest: PackManifest,
  files: EditorFile[]
): DependencyEdge[] {
  const edges: DependencyEdge[] = [];

  // Agents depend on profiles, adapters, toolsets, and files
  for (const [name, agent] of Object.entries(manifest.agents ?? {})) {
    const agentId = `agents.${name}`;

    edges.push({
      from: agentId,
      to: `profiles.${agent.profile}`,
      type: 'profile',
    });

    if (agent.adapter) {
      edges.push({
        from: agentId,
        to: `adapters.${agent.adapter}`,
        type: 'adapter',
      });
    }

    for (const ts of agent.toolsets ?? []) {
      edges.push({
        from: agentId,
        to: `toolsets.${ts}`,
        type: 'toolset',
      });
    }

    edges.push({
      from: agentId,
      to: agent.prompt_md,
      type: 'file',
    });
  }

  // Toolsets depend on tools
  for (const [name, toolset] of Object.entries(manifest.toolsets ?? {})) {
    const toolsetId = `toolsets.${name}`;

    for (const tool of toolset.tools) {
      edges.push({
        from: toolsetId,
        to: tool,
        type: 'tool',
      });
    }
  }

  // Routing depends on providers
  if (manifest.routing?.embedding_provider) {
    edges.push({
      from: 'routing',
      to: `providers.${manifest.routing.embedding_provider}`,
      type: 'provider',
    });
  }

  if (manifest.routing?.summarization_provider) {
    edges.push({
      from: 'routing',
      to: `providers.${manifest.routing.summarization_provider}`,
      type: 'provider',
    });
  }

  // Profiles depend on providers (embeddings)
  for (const [name, profile] of Object.entries(manifest.profiles ?? {})) {
    edges.push({
      from: `profiles.${name}`,
      to: `providers.${profile.embeddings}`,
      type: 'provider',
    });
  }

  return edges;
}

export type PackStore = ReturnType<typeof usePackStore>;
