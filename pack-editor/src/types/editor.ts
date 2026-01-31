/**
 * Editor state types
 */

export type ValidationStage = 'idle' | 'parse' | 'schema' | 'refs' | 'server' | 'ready';

export type FileLanguage = 'toml' | 'markdown' | 'json' | 'yaml';

export interface EditorFile {
  path: string;
  content: string;
  originalContent: string; // For dirty detection
  isDirty: boolean;
  language: FileLanguage;
}

export interface FileTreeNode {
  name: string;
  path: string;
  type: 'file' | 'folder';
  children?: FileTreeNode[];
  language?: FileLanguage;
  isDirty?: boolean;
}

export interface DependencyEdge {
  from: string;
  to: string;
  type: 'profile' | 'adapter' | 'toolset' | 'tool' | 'provider' | 'file' | 'injection';
}

export interface DependencyNode {
  id: string;
  type: 'agent' | 'profile' | 'toolset' | 'tool' | 'provider' | 'adapter' | 'file' | 'injection';
  label: string;
}

export interface ValidationResult {
  stage: ValidationStage;
  errors: import('./api').PackDiagnostic[];
  warnings: import('./api').PackDiagnostic[];
}

// CodeMirror editor ref type
export interface EditorRef {
  getContent: () => string;
  setContent: (content: string) => void;
  focus: () => void;
}
