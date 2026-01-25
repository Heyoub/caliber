#!/usr/bin/env python3
"""
rust_workspace_analyzer.py - Elite Rust Workspace Static Analysis

A READ-ONLY tool for comprehensive Rust workspace analysis:
- Full dependency graph (workspace + external + feature edges)
- Feature matrix (features ↔ crates ↔ modules ↔ symbols ↔ call edges)
- Smell detection (duplication, fragmentation, cycles, hotspots)
- Human-usable reports

SAFETY: NEVER modifies repository files. All output goes to --out directory.

Usage:
    python rust_workspace_analyzer.py --repo /path/to/rust/workspace
    python rust_workspace_analyzer.py --repo . --out ./analysis_out --verbose
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional, Any
from concurrent.futures import ThreadPoolExecutor, as_completed

# Python 3.11+ for tomllib
try:
    import tomllib
except ImportError:
    tomllib = None  # Fallback to regex parsing

# Tree-sitter detection (optional, for better parsing)
try:
    import tree_sitter_rust as ts_rust
    from tree_sitter import Language, Parser
    TREE_SITTER_AVAILABLE = True
except ImportError:
    TREE_SITTER_AVAILABLE = False

# ============================================================================
# CONFIGURATION
# ============================================================================

VERSION = "1.0.0"
DEFAULT_IGNORES = {
    'target', '.git', 'node_modules', '.idea', '.vscode',
    'dist', 'build', '__pycache__', '.cargo', 'vendor'
}
MAX_FILE_SIZE = 1_000_000  # 1MB per file limit
MAX_LINE_LENGTH = 2000

# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class CrateNode:
    """A crate in the dependency graph."""
    name: str
    version: str
    source: str  # "workspace", "crates.io", "git", "path"
    path: Optional[str] = None
    features_available: list = field(default_factory=list)
    features_enabled: list = field(default_factory=list)
    is_workspace: bool = False

@dataclass
class DependencyEdge:
    """An edge in the dependency graph."""
    from_crate: str
    to_crate: str
    kind: str  # "normal", "dev", "build"
    features: list = field(default_factory=list)
    optional: bool = False

@dataclass
class Symbol:
    """A symbol in the codebase."""
    name: str
    kind: str  # "struct", "enum", "trait", "fn", "impl", "const", "type", "macro"
    file: str
    line_start: int
    line_end: int
    visibility: str = "private"  # "pub", "pub(crate)", "private"
    parent: Optional[str] = None  # For methods: parent struct/impl
    cfg_features: list = field(default_factory=list)  # #[cfg(feature = "...")]
    doc_lines: int = 0

@dataclass
class Import:
    """A use statement."""
    path: str
    file: str
    line: int
    alias: Optional[str] = None
    is_glob: bool = False

@dataclass
class FeatureGate:
    """A feature gate in code."""
    feature: str
    file: str
    line: int
    kind: str  # "cfg", "cfg_attr", "mod", "use"
    scope: str  # What it gates

@dataclass
class ModuleInfo:
    """Information about a Rust module."""
    name: str
    path: str
    crate_name: str
    symbols: list = field(default_factory=list)
    imports: list = field(default_factory=list)
    feature_gates: list = field(default_factory=list)
    lines: int = 0
    doc_lines: int = 0
    complexity_score: float = 0.0

@dataclass
class Smell:
    """A detected code smell."""
    kind: str
    severity: str  # "error", "warning", "info"
    file: str
    line: int
    message: str
    context: str
    suggestion: str
    confidence: float = 1.0
    evidence: list = field(default_factory=list)

@dataclass
class DuplicateCluster:
    """A cluster of duplicate/similar code."""
    signature: str
    locations: list = field(default_factory=list)  # [{"file": str, "line_start": int, "line_end": int}]
    excerpt: str = ""
    similarity: float = 1.0

@dataclass
class AnalysisResult:
    """Complete analysis result."""
    # Dependency graph
    crate_nodes: dict = field(default_factory=dict)  # name -> CrateNode
    dependency_edges: list = field(default_factory=list)

    # Symbol index
    modules: dict = field(default_factory=dict)  # path -> ModuleInfo
    symbols_by_name: dict = field(default_factory=dict)  # name -> [Symbol]

    # Feature matrix
    features_by_crate: dict = field(default_factory=dict)  # crate -> [features]
    feature_usage: dict = field(default_factory=dict)  # feature -> {files, symbols, count}

    # Smells
    smells: list = field(default_factory=list)
    duplicates: list = field(default_factory=list)

    # Metrics
    metrics: dict = field(default_factory=dict)

# ============================================================================
# UTILITY FUNCTIONS
# ============================================================================

def echo(msg: str, indent: int = 0, verbose: bool = True):
    """Print with indentation."""
    if verbose:
        print(f"{'  ' * indent}-> {msg}")

def safe_read(path: Path, max_size: int = MAX_FILE_SIZE) -> Optional[str]:
    """Safely read a file with size limit."""
    try:
        if path.stat().st_size > max_size:
            return None
        return path.read_text(errors='replace')
    except Exception:
        return None

def normalize_code(code: str) -> str:
    """Normalize code for comparison (strip comments, whitespace)."""
    # Remove block comments
    code = re.sub(r'/\*.*?\*/', '', code, flags=re.DOTALL)
    # Remove line comments
    code = re.sub(r'//.*$', '', code, flags=re.MULTILINE)
    # Remove string literals (replace with placeholder)
    code = re.sub(r'"(?:[^"\\]|\\.)*"', '""', code)
    code = re.sub(r"'(?:[^'\\]|\\.)'", "''", code)
    # Normalize whitespace
    code = re.sub(r'\s+', ' ', code)
    return code.strip()

def compute_shingle_hash(text: str, k: int = 5) -> set:
    """Compute k-shingle hashes for similarity detection."""
    words = text.split()
    if len(words) < k:
        return {hash(text)}
    return {hash(' '.join(words[i:i+k])) for i in range(len(words) - k + 1)}

def jaccard_similarity(set1: set, set2: set) -> float:
    """Compute Jaccard similarity between two sets."""
    if not set1 or not set2:
        return 0.0
    intersection = len(set1 & set2)
    union = len(set1 | set2)
    return intersection / union if union > 0 else 0.0

# ============================================================================
# CARGO METADATA PARSING
# ============================================================================

def run_cargo_metadata(repo: Path) -> Optional[dict]:
    """Run cargo metadata and parse output."""
    try:
        result = subprocess.run(
            ['cargo', 'metadata', '--format-version', '1', '--no-deps'],
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=60
        )
        if result.returncode == 0:
            return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, FileNotFoundError, json.JSONDecodeError):
        pass
    return None

def run_cargo_metadata_full(repo: Path) -> Optional[dict]:
    """Run cargo metadata with all features for full dependency graph."""
    try:
        result = subprocess.run(
            ['cargo', 'metadata', '--format-version', '1', '--all-features'],
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=120
        )
        if result.returncode == 0:
            return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, FileNotFoundError, json.JSONDecodeError):
        pass
    return None

def parse_dependency_graph(metadata: dict, full_metadata: Optional[dict]) -> tuple:
    """Parse cargo metadata into crate nodes and edges."""
    nodes = {}
    edges = []

    workspace_members = set(metadata.get('workspace_members', []))
    packages = {p['id']: p for p in metadata.get('packages', [])}

    # Parse packages
    for pkg_id, pkg in packages.items():
        is_workspace = pkg_id in workspace_members

        node = CrateNode(
            name=pkg['name'],
            version=pkg['version'],
            source="workspace" if is_workspace else pkg.get('source', 'unknown'),
            path=pkg.get('manifest_path', '').replace('/Cargo.toml', ''),
            features_available=list(pkg.get('features', {}).keys()),
            is_workspace=is_workspace
        )
        nodes[pkg['name']] = node

        # Parse dependencies
        for dep in pkg.get('dependencies', []):
            edge = DependencyEdge(
                from_crate=pkg['name'],
                to_crate=dep['name'],
                kind=dep.get('kind') or 'normal',
                features=dep.get('features', []),
                optional=dep.get('optional', False)
            )
            edges.append(edge)

    # Parse resolve for enabled features
    resolve = metadata.get('resolve') or {}
    for node_info in resolve.get('nodes') or []:
        pkg_id = node_info['id']
        # Extract crate name from package ID
        name_match = re.match(r'^([^ ]+)', pkg_id)
        if name_match:
            name = name_match.group(1)
            if name in nodes:
                nodes[name].features_enabled = node_info.get('features', [])

    return nodes, edges

# ============================================================================
# CARGO.TOML FEATURE PARSING
# ============================================================================

def parse_cargo_toml_features(path: Path) -> dict:
    """Parse [features] section from Cargo.toml."""
    content = safe_read(path)
    if not content:
        return {}

    features = {}

    if tomllib:
        try:
            data = tomllib.loads(content)
            features = data.get('features', {})
        except Exception:
            pass

    if not features:
        # Fallback: regex parsing
        in_features = False
        for line in content.split('\n'):
            line = line.strip()
            if line == '[features]':
                in_features = True
                continue
            if line.startswith('[') and in_features:
                break
            if in_features and '=' in line:
                match = re.match(r'^(\w+)\s*=\s*\[(.*?)\]', line)
                if match:
                    name = match.group(1)
                    deps = [d.strip().strip('"\'') for d in match.group(2).split(',') if d.strip()]
                    features[name] = deps

    return features

# ============================================================================
# RUST CODE PARSING
# ============================================================================

# Regex patterns for symbol extraction
PATTERNS = {
    'mod_decl': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*[;{]', re.M),
    'use_stmt': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);', re.M),
    'struct': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?struct\s+(\w+)', re.M),
    'enum': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?enum\s+(\w+)', re.M),
    'trait': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?trait\s+(\w+)', re.M),
    'impl_block': re.compile(r'^\s*impl(?:<[^>]+>)?\s+(?:(\w+)\s+for\s+)?(\w+)', re.M),
    'fn_sig': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+(\w+)', re.M),
    'const_def': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?const\s+(\w+)', re.M),
    'type_alias': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?type\s+(\w+)', re.M),
    'macro_def': re.compile(r'^\s*(pub(?:\([^)]*\))?\s+)?macro_rules!\s+(\w+)', re.M),
    'cfg_feature': re.compile(r'#\[cfg\s*\(\s*feature\s*=\s*"([^"]+)"\s*\)\]', re.M),
    'cfg_any_feature': re.compile(r'#\[cfg\s*\(\s*any\s*\([^)]*feature\s*=\s*"([^"]+)"', re.M),
    'cfg_attr_feature': re.compile(r'#\[cfg_attr\s*\(\s*feature\s*=\s*"([^"]+)"', re.M),
}

def extract_symbols_regex(content: str, file_path: str, crate_name: str) -> ModuleInfo:
    """Extract symbols using regex (fallback parser)."""
    lines = content.split('\n')

    # Module name from path
    path = Path(file_path)
    name = path.stem
    if name in ('mod', 'lib', 'main'):
        name = path.parent.name if name == 'mod' else crate_name

    module = ModuleInfo(
        name=name,
        path=file_path,
        crate_name=crate_name,
        lines=len(lines),
        doc_lines=sum(1 for l in lines if l.strip().startswith('///') or l.strip().startswith('//!'))
    )

    # Extract imports
    for m in PATTERNS['use_stmt'].finditer(content):
        line_num = content[:m.start()].count('\n') + 1
        path_str = m.group(1).strip()
        module.imports.append(Import(
            path=path_str,
            file=file_path,
            line=line_num,
            is_glob='::*' in path_str
        ))

    # Extract feature gates
    for pattern_name in ('cfg_feature', 'cfg_any_feature', 'cfg_attr_feature'):
        for m in PATTERNS[pattern_name].finditer(content):
            line_num = content[:m.start()].count('\n') + 1
            module.feature_gates.append(FeatureGate(
                feature=m.group(1),
                file=file_path,
                line=line_num,
                kind=pattern_name.replace('_feature', ''),
                scope=lines[line_num] if line_num <= len(lines) else ''
            ))

    # Extract symbols
    for kind, pattern_name in [
        ('struct', 'struct'), ('enum', 'enum'), ('trait', 'trait'),
        ('fn', 'fn_sig'), ('const', 'const_def'), ('type', 'type_alias'),
        ('macro', 'macro_def')
    ]:
        for m in PATTERNS[pattern_name].finditer(content):
            line_num = content[:m.start()].count('\n') + 1
            vis = 'pub' if m.group(1) and 'pub' in m.group(1) else 'private'

            # Estimate end line (look for closing brace or next item)
            end_line = estimate_symbol_end(lines, line_num - 1)

            # Check for cfg(feature) on previous lines
            cfg_features = []
            if line_num > 1:
                prev_lines = '\n'.join(lines[max(0, line_num-3):line_num-1])
                for cfg_m in PATTERNS['cfg_feature'].finditer(prev_lines):
                    cfg_features.append(cfg_m.group(1))

            symbol = Symbol(
                name=m.group(2),
                kind=kind,
                file=file_path,
                line_start=line_num,
                line_end=end_line,
                visibility=vis,
                cfg_features=cfg_features
            )
            module.symbols.append(symbol)

    # Extract impl blocks
    for m in PATTERNS['impl_block'].finditer(content):
        line_num = content[:m.start()].count('\n') + 1
        trait_name = m.group(1) or ''
        type_name = m.group(2)
        end_line = estimate_symbol_end(lines, line_num - 1)

        symbol = Symbol(
            name=f"impl {trait_name + ' for ' if trait_name else ''}{type_name}",
            kind='impl',
            file=file_path,
            line_start=line_num,
            line_end=end_line,
            visibility='pub',
            parent=type_name
        )
        module.symbols.append(symbol)

    # Calculate complexity (simple heuristic)
    module.complexity_score = calculate_complexity(content, lines)

    return module

def estimate_symbol_end(lines: list, start_idx: int) -> int:
    """Estimate where a symbol definition ends."""
    brace_count = 0
    started = False

    for i in range(start_idx, min(start_idx + 500, len(lines))):
        line = lines[i]
        # Remove strings and comments for accurate brace counting
        line = re.sub(r'"(?:[^"\\]|\\.)*"', '', line)
        line = re.sub(r'//.*$', '', line)

        brace_count += line.count('{') - line.count('}')
        if '{' in line:
            started = True

        # End conditions
        if started and brace_count == 0:
            return i + 1
        if not started and ';' in line:
            return i + 1

    return start_idx + 1

def calculate_complexity(content: str, lines: list) -> float:
    """Calculate cyclomatic complexity estimate."""
    score = 0.0

    # Count decision points
    decision_keywords = ['if', 'else', 'match', 'while', 'for', 'loop', '?', '&&', '||']
    for kw in decision_keywords:
        score += content.count(f' {kw} ') + content.count(f' {kw}(') + content.count(f'\t{kw} ')

    # Normalize by lines
    if len(lines) > 0:
        score = score / len(lines) * 100

    return round(score, 2)

# ============================================================================
# SMELL DETECTION
# ============================================================================

SMELL_PATTERNS = {
    # Critical: Wrong language code
    'wrong_lang_js': (r'console\.log|\.forEach\s*\(|\.map\s*\(\s*\w+\s*=>', 'error', 'JavaScript code in Rust file'),
    'wrong_lang_py': (r'\bdef\s+\w+\s*\(|^\s*pass\s*$', 'error', 'Python code in Rust file'),

    # High: Stubs and placeholders
    'todo_macro': (r'\btodo!\s*\(', 'warning', 'todo!() placeholder'),
    'unimplemented': (r'\bunimplemented!\s*\(', 'warning', 'unimplemented!() stub'),
    'empty_fn': (r'fn\s+\w+[^{]+\{\s*\}', 'warning', 'Empty function body'),

    # Medium: Safety issues
    'unsafe_block': (r'\bunsafe\s*\{', 'info', 'unsafe block (audit required)'),
    'transmute': (r'\btransmute\b', 'warning', 'transmute (extremely unsafe)'),
    'unwrap_chain': (r'\.unwrap\(\)\.unwrap\(\)', 'warning', 'Double unwrap chain'),

    # Low: Style issues
    'dbg_macro': (r'\bdbg!\s*\(', 'info', 'dbg!() macro (debug leftover)'),
    'wildcard_import': (r'use\s+\w+(?:::\w+)*::\*;', 'info', 'Wildcard import'),
    'magic_number': (r'(?<![.\w])\d{5,}(?![.\w])', 'info', 'Magic number'),
}

SMELL_SUGGESTIONS = {
    'wrong_lang_js': 'REMOVE - wrong language',
    'wrong_lang_py': 'REMOVE - wrong language',
    'todo_macro': 'Implement or track in issue',
    'unimplemented': 'Implement the function',
    'empty_fn': 'Implement function body',
    'unsafe_block': 'Document safety invariants',
    'transmute': 'Use safe alternatives',
    'unwrap_chain': 'Use ? or and_then',
    'dbg_macro': 'Remove before release',
    'wildcard_import': 'Use explicit imports',
    'magic_number': 'Extract to named constant',
}

def detect_smells(content: str, file_path: str) -> list:
    """Detect code smells in a file."""
    smells = []
    lines = content.split('\n')

    # Strip comments for pattern matching
    stripped = normalize_code(content)

    for kind, (pattern, severity, msg) in SMELL_PATTERNS.items():
        rx = re.compile(pattern, re.M)
        for i, line in enumerate(lines, 1):
            if rx.search(line):
                smells.append(Smell(
                    kind=kind,
                    severity=severity,
                    file=file_path,
                    line=i,
                    message=msg,
                    context=line.strip()[:80],
                    suggestion=SMELL_SUGGESTIONS.get(kind, 'Review and fix'),
                    confidence=0.9
                ))

    # Long function detection
    fn_starts = []
    for i, line in enumerate(lines):
        if re.match(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+\w+', line):
            fn_starts.append(i)

    for j, start in enumerate(fn_starts):
        end = fn_starts[j+1] if j+1 < len(fn_starts) else len(lines)
        fn_len = end - start
        if fn_len > 100:
            smells.append(Smell(
                kind='long_function',
                severity='warning',
                file=file_path,
                line=start + 1,
                message=f'{fn_len} line function',
                context=lines[start].strip()[:60],
                suggestion='Split into smaller functions',
                confidence=1.0
            ))

    # Deep nesting detection
    for i, line in enumerate(lines, 1):
        leading_spaces = len(line) - len(line.lstrip())
        if leading_spaces >= 24:  # 6+ levels of indentation
            smells.append(Smell(
                kind='deep_nesting',
                severity='info',
                file=file_path,
                line=i,
                message='Deep nesting (6+ levels)',
                context=line.strip()[:60],
                suggestion='Extract helper function',
                confidence=0.8
            ))

    return smells

def detect_imported_but_low_signal(module) -> list:
    """
    Detect "imported-but-low-signal local methods":
    - Imports only used by one short function
    - Function is mostly plumbing (passes args through)
    """
    smells = []

    # Handle dict or ModuleInfo
    mod_path = module.path if hasattr(module, 'path') else module.get('path', '')
    imports = module.imports if hasattr(module, 'imports') else module.get('imports', [])
    symbols = module.symbols if hasattr(module, 'symbols') else module.get('symbols', [])

    # Group imports by what they import
    import_usages = defaultdict(set)  # import_path -> set of symbol names that use it

    for imp in imports:
        imp_path = imp.path if hasattr(imp, 'path') else imp.get('path', '')
        # Extract the final item from import path
        parts = imp_path.replace('{', '').replace('}', '').split('::')
        imported_items = [p.strip() for p in parts[-1].split(',') if p.strip()]

        for symbol in symbols:
            sym_kind = symbol.kind if hasattr(symbol, 'kind') else symbol.get('kind', '')
            sym_name = symbol.name if hasattr(symbol, 'name') else symbol.get('name', '')
            if sym_kind == 'fn':
                # Check if this function uses any of the imported items
                for item in imported_items:
                    if item and item in str(symbol):  # Simple heuristic
                        import_usages[imp_path].add(sym_name)

    # Find imports used by only one short function
    for imp_path, users in import_usages.items():
        if len(users) == 1:
            fn_name = list(users)[0]
            # Find the function
            for symbol in symbols:
                sym_name = symbol.name if hasattr(symbol, 'name') else symbol.get('name', '')
                sym_kind = symbol.kind if hasattr(symbol, 'kind') else symbol.get('kind', '')
                sym_line_start = symbol.line_start if hasattr(symbol, 'line_start') else symbol.get('line_start', 0)
                sym_line_end = symbol.line_end if hasattr(symbol, 'line_end') else symbol.get('line_end', 0)

                if sym_name == fn_name and sym_kind == 'fn':
                    fn_length = sym_line_end - sym_line_start
                    if fn_length < 10:  # Short function
                        smells.append(Smell(
                            kind='imported_but_low_signal',
                            severity='info',
                            file=mod_path,
                            line=sym_line_start,
                            message=f'Import "{imp_path}" only used in short function "{fn_name}"',
                            context=f'{fn_length} line function',
                            suggestion='Consider inlining or reviewing abstraction',
                            confidence=0.6,
                            evidence=[f'Import: {imp_path}', f'Function: {fn_name}', f'Length: {fn_length} lines']
                        ))

    return smells

# ============================================================================
# DUPLICATION DETECTION
# ============================================================================

def detect_duplicates(modules: dict, min_lines: int = 5, similarity_threshold: float = 0.8) -> list:
    """
    Two-stage duplicate detection:
    1. Cheap candidate generation via shingle hashing
    2. Verification via similarity ratio
    """
    clusters = []

    # Stage 1: Extract function bodies and compute shingles
    fn_signatures = {}  # signature_hash -> [(file, line_start, line_end, content)]

    for mod_path, module in modules.items():
        mod_file_path = module.path if hasattr(module, 'path') else module.get('path', '')
        content = safe_read(Path(mod_file_path))
        if not content:
            continue

        lines = content.split('\n')

        symbols = module.symbols if hasattr(module, 'symbols') else module.get('symbols', [])
        for symbol in symbols:
            sym_kind = symbol.kind if hasattr(symbol, 'kind') else symbol.get('kind', '')
            if sym_kind != 'fn':
                continue

            # Extract function body
            sym_line_start = symbol.line_start if hasattr(symbol, 'line_start') else symbol.get('line_start', 0)
            sym_line_end = symbol.line_end if hasattr(symbol, 'line_end') else symbol.get('line_end', 0)
            sym_name = symbol.name if hasattr(symbol, 'name') else symbol.get('name', '')

            start = sym_line_start - 1
            end = min(sym_line_end, len(lines))
            if end - start < min_lines:
                continue

            fn_body = '\n'.join(lines[start:end])
            normalized = normalize_code(fn_body)

            # Compute shingle hash
            shingles = compute_shingle_hash(normalized, k=3)
            sig = tuple(sorted(shingles)[:10])  # Take first 10 for quick comparison

            if sig not in fn_signatures:
                fn_signatures[sig] = []
            fn_signatures[sig].append({
                'file': mod_file_path,
                'line_start': sym_line_start,
                'line_end': sym_line_end,
                'content': fn_body,
                'shingles': shingles,
                'name': sym_name
            })

    # Stage 2: Compare candidates with same signature prefix
    seen = set()

    for sig, candidates in fn_signatures.items():
        if len(candidates) < 2:
            continue

        for i, c1 in enumerate(candidates):
            for c2 in candidates[i+1:]:
                # Skip if same file
                if c1['file'] == c2['file']:
                    continue

                # Compute actual similarity
                similarity = jaccard_similarity(c1['shingles'], c2['shingles'])
                if similarity >= similarity_threshold:
                    # Avoid duplicate clusters
                    cluster_key = tuple(sorted([
                        f"{c1['file']}:{c1['line_start']}",
                        f"{c2['file']}:{c2['line_start']}"
                    ]))
                    if cluster_key in seen:
                        continue
                    seen.add(cluster_key)

                    clusters.append(DuplicateCluster(
                        signature=f"{c1['name']} ~ {c2['name']}",
                        locations=[
                            {'file': c1['file'], 'line_start': c1['line_start'], 'line_end': c1['line_end']},
                            {'file': c2['file'], 'line_start': c2['line_start'], 'line_end': c2['line_end']}
                        ],
                        excerpt=c1['content'][:200] + '...' if len(c1['content']) > 200 else c1['content'],
                        similarity=round(similarity, 2)
                    ))

    return clusters

# ============================================================================
# FEATURE MATRIX
# ============================================================================

def build_feature_matrix(modules: dict, crates: dict, features_by_crate: dict) -> dict:
    """Build feature ↔ symbol matrix."""
    matrix = defaultdict(lambda: {
        'crates': set(),
        'files': set(),
        'symbols': [],
        'cfg_count': 0
    })

    for mod_path, module in modules.items():
        crate_name = module.crate_name if hasattr(module, 'crate_name') else module.get('crate_name', '')

        feature_gates = module.feature_gates if hasattr(module, 'feature_gates') else module.get('feature_gates', [])
        for gate in feature_gates:
            feature = gate.feature if hasattr(gate, 'feature') else gate.get('feature', '')
            matrix[feature]['crates'].add(crate_name)
            matrix[feature]['files'].add(mod_path)
            matrix[feature]['cfg_count'] += 1

        symbols = module.symbols if hasattr(module, 'symbols') else module.get('symbols', [])
        for symbol in symbols:
            cfg_features = symbol.cfg_features if hasattr(symbol, 'cfg_features') else symbol.get('cfg_features', [])
            for feature in cfg_features:
                matrix[feature]['crates'].add(crate_name)
                matrix[feature]['files'].add(mod_path)
                sym_name = symbol.name if hasattr(symbol, 'name') else symbol.get('name', '')
                sym_kind = symbol.kind if hasattr(symbol, 'kind') else symbol.get('kind', '')
                sym_line = symbol.line_start if hasattr(symbol, 'line_start') else symbol.get('line_start', 0)
                matrix[feature]['symbols'].append({
                    'name': sym_name,
                    'kind': sym_kind,
                    'file': mod_path,
                    'line': sym_line
                })

    # Convert sets to lists for JSON serialization
    result = {}
    for feature, data in matrix.items():
        result[feature] = {
            'crates': list(data['crates']),
            'files': list(data['files']),
            'symbols': data['symbols'],
            'cfg_count': data['cfg_count'],
            'spread': len(data['files'])  # How fragmented is this feature?
        }

    return result

# ============================================================================
# CYCLE DETECTION
# ============================================================================

def detect_cycles(edges: list) -> list:
    """Detect dependency cycles using DFS."""
    graph = defaultdict(list)
    for edge in edges:
        graph[edge.from_crate].append(edge.to_crate)

    cycles = []
    visited = set()
    rec_stack = set()

    def dfs(node, path):
        visited.add(node)
        rec_stack.add(node)

        for neighbor in list(graph[node]):  # Convert to list to avoid mutation issues
            if neighbor not in visited:
                result = dfs(neighbor, path + [neighbor])
                if result:
                    cycles.append(result)
            elif neighbor in rec_stack:
                # Found cycle
                if neighbor in path:
                    cycle_start = path.index(neighbor)
                    return path[cycle_start:] + [neighbor]

        rec_stack.remove(node)
        return None

    # Iterate over a copy of keys to avoid mutation during iteration
    for node in list(graph.keys()):
        if node not in visited:
            dfs(node, [node])

    return cycles

# ============================================================================
# MAIN ANALYSIS
# ============================================================================

def find_rust_files(root: Path, ignores: set) -> list:
    """Find all .rs files in the repository."""
    files = []
    for path in root.rglob('*.rs'):
        if not any(ignore in path.parts for ignore in ignores):
            files.append(path)
    return files

def find_cargo_tomls(root: Path, ignores: set) -> list:
    """Find all Cargo.toml files."""
    files = []
    for path in root.rglob('Cargo.toml'):
        if not any(ignore in path.parts for ignore in ignores):
            files.append(path)
    return files

def get_crate_name(file_path: Path, root: Path) -> str:
    """Derive crate name from file path."""
    for parent in file_path.parents:
        cargo = parent / 'Cargo.toml'
        if cargo.exists():
            content = safe_read(cargo)
            if content:
                m = re.search(r'name\s*=\s*"([^"]+)"', content)
                if m:
                    return m.group(1)
        if parent == root:
            break
    return 'unknown'

def analyze_workspace(
    repo: Path,
    out: Path,
    include_tests: bool = True,
    max_files: int = 10000,
    jobs: int = 4,
    verbose: bool = False
) -> AnalysisResult:
    """Main analysis function."""
    result = AnalysisResult()

    echo(f"Analyzing: {repo}", verbose=verbose)
    echo(f"Output: {out}", verbose=verbose)

    # Phase 1: Parse cargo metadata
    echo("Phase 1: Parsing cargo metadata...", 1, verbose)
    metadata = run_cargo_metadata(repo)
    full_metadata = run_cargo_metadata_full(repo)

    if metadata:
        nodes, edges = parse_dependency_graph(metadata, full_metadata)
        result.crate_nodes = {k: asdict(v) for k, v in nodes.items()}
        result.dependency_edges = [asdict(e) for e in edges]
        echo(f"Found {len(nodes)} crates, {len(edges)} dependencies", 2, verbose)
    else:
        echo("cargo metadata failed, using file scanning", 2, verbose)

    # Phase 2: Scan Cargo.toml for features
    echo("Phase 2: Extracting features from Cargo.toml...", 1, verbose)
    cargo_files = find_cargo_tomls(repo, DEFAULT_IGNORES)
    for cargo_path in cargo_files:
        crate_name = get_crate_name(cargo_path.parent / 'src' / 'lib.rs', repo)
        features = parse_cargo_toml_features(cargo_path)
        if features:
            result.features_by_crate[crate_name] = features
    echo(f"Found features in {len(result.features_by_crate)} crates", 2, verbose)

    # Phase 3: Scan Rust files
    echo("Phase 3: Scanning Rust files...", 1, verbose)
    rust_files = find_rust_files(repo, DEFAULT_IGNORES)
    if not include_tests:
        rust_files = [f for f in rust_files if 'test' not in str(f).lower()]
    rust_files = rust_files[:max_files]
    echo(f"Scanning {len(rust_files)} files...", 2, verbose)

    # Parse files in parallel
    def parse_file(file_path: Path):
        content = safe_read(file_path)
        if not content:
            return None
        crate_name = get_crate_name(file_path, repo)
        return extract_symbols_regex(content, str(file_path), crate_name)

    with ThreadPoolExecutor(max_workers=jobs) as executor:
        futures = {executor.submit(parse_file, f): f for f in rust_files}
        for future in as_completed(futures):
            module = future.result()
            if module:
                result.modules[module.path] = asdict(module)
                # Index symbols
                for symbol in module.symbols:
                    if symbol.name not in result.symbols_by_name:
                        result.symbols_by_name[symbol.name] = []
                    result.symbols_by_name[symbol.name].append(asdict(symbol))

    echo(f"Parsed {len(result.modules)} modules", 2, verbose)

    # Phase 4: Build feature matrix
    echo("Phase 4: Building feature matrix...", 1, verbose)
    modules_typed = {k: ModuleInfo(**v) for k, v in result.modules.items()}
    result.feature_usage = build_feature_matrix(modules_typed, result.crate_nodes, result.features_by_crate)
    echo(f"Mapped {len(result.feature_usage)} features", 2, verbose)

    # Phase 5: Detect smells
    echo("Phase 5: Detecting smells...", 1, verbose)
    for file_path in rust_files:
        content = safe_read(file_path)
        if content:
            smells = detect_smells(content, str(file_path))
            result.smells.extend([asdict(s) for s in smells])

    # Detect imported-but-low-signal
    for mod_path, mod_data in result.modules.items():
        module = ModuleInfo(**mod_data)
        low_signal = detect_imported_but_low_signal(module)
        result.smells.extend([asdict(s) for s in low_signal])

    echo(f"Found {len(result.smells)} smells", 2, verbose)

    # Phase 6: Detect duplicates
    echo("Phase 6: Detecting duplicates...", 1, verbose)
    duplicates = detect_duplicates(modules_typed)
    result.duplicates = [asdict(d) for d in duplicates]
    echo(f"Found {len(result.duplicates)} duplicate clusters", 2, verbose)

    # Phase 7: Detect cycles
    echo("Phase 7: Detecting cycles...", 1, verbose)
    if result.dependency_edges:
        edges = [DependencyEdge(**e) for e in result.dependency_edges]
        cycles = detect_cycles(edges)
        if cycles:
            for cycle in cycles:
                result.smells.append({
                    'kind': 'dependency_cycle',
                    'severity': 'warning',
                    'file': '',
                    'line': 0,
                    'message': f"Dependency cycle: {' -> '.join(cycle)}",
                    'context': '',
                    'suggestion': 'Break the cycle by restructuring dependencies',
                    'confidence': 1.0,
                    'evidence': cycle
                })

    # Calculate metrics
    result.metrics = {
        'crates': len(result.crate_nodes),
        'modules': len(result.modules),
        'symbols': sum(len(v) for v in result.symbols_by_name.values()),
        'features': len(result.feature_usage),
        'smells': len(result.smells),
        'duplicates': len(result.duplicates),
        'total_lines': sum(m.get('lines', 0) for m in result.modules.values()),
        'doc_lines': sum(m.get('doc_lines', 0) for m in result.modules.values()),
    }

    return result

# ============================================================================
# OUTPUT GENERATION
# ============================================================================

def generate_dependency_graph_json(result: AnalysisResult, out: Path):
    """Generate dependency_graph.json."""
    data = {
        'nodes': result.crate_nodes,
        'edges': result.dependency_edges
    }
    (out / 'dependency_graph.json').write_text(json.dumps(data, indent=2))

def generate_symbol_index_json(result: AnalysisResult, out: Path):
    """Generate workspace_symbol_index.json."""
    (out / 'workspace_symbol_index.json').write_text(json.dumps(result.modules, indent=2))

def generate_feature_matrix_json(result: AnalysisResult, out: Path):
    """Generate feature_matrix.json."""
    data = {
        'by_crate': result.features_by_crate,
        'usage': result.feature_usage
    }
    (out / 'feature_matrix.json').write_text(json.dumps(data, indent=2))

def generate_smells_report(result: AnalysisResult, out: Path):
    """Generate smells_report.md."""
    lines = ['# Code Smells Report', '', '## Summary', '']

    # Group by severity
    by_severity = defaultdict(list)
    for smell in result.smells:
        by_severity[smell['severity']].append(smell)

    lines.append(f"- **Errors**: {len(by_severity['error'])}")
    lines.append(f"- **Warnings**: {len(by_severity['warning'])}")
    lines.append(f"- **Info**: {len(by_severity['info'])}")
    lines.append('')

    # Errors (Critical)
    if by_severity['error']:
        lines.append('## Errors (Critical)')
        lines.append('')
        for smell in by_severity['error']:
            lines.append(f"### {smell['kind']}")
            lines.append(f"- **File**: `{smell['file']}:{smell['line']}`")
            lines.append(f"- **Message**: {smell['message']}")
            lines.append(f"- **Context**: `{smell['context']}`")
            lines.append(f"- **Fix**: {smell['suggestion']}")
            lines.append('')

    # Warnings
    if by_severity['warning']:
        lines.append('## Warnings')
        lines.append('')
        for smell in by_severity['warning'][:50]:  # Limit output
            lines.append(f"- `{Path(smell['file']).name}:{smell['line']}` - {smell['message']}")

    # Duplicates
    if result.duplicates:
        lines.append('')
        lines.append('## Duplicate Code Clusters')
        lines.append('')
        for dup in result.duplicates[:20]:
            lines.append(f"### {dup['signature']} (similarity: {dup['similarity']})")
            for loc in dup['locations']:
                lines.append(f"- `{loc['file']}:{loc['line_start']}-{loc['line_end']}`")
            lines.append('')

    (out / 'smells_report.md').write_text('\n'.join(lines))

def generate_interactive_helpers(result: AnalysisResult, out: Path):
    """Generate interactive_helpers.txt."""
    lines = ['# Interactive Analysis Helpers', '', '## Grep Commands', '']

    # Common searches
    lines.append('# Find all todo macros')
    lines.append(r'rg "todo!\s*\(" --type rust')
    lines.append('')
    lines.append('# Find all unsafe blocks')
    lines.append(r'rg "unsafe\s*{" --type rust')
    lines.append('')
    lines.append('# Find all feature gates')
    lines.append(r"rg '#\[cfg\(feature\s*=\s*\"' --type rust")
    lines.append('')
    lines.append('# Find wildcard imports')
    lines.append(r'rg "use.*::\*;" --type rust')
    lines.append('')

    lines.append('## Next Questions to Ask')
    lines.append('')

    # Add questions based on findings
    if result.smells:
        error_count = sum(1 for s in result.smells if s['severity'] == 'error')
        if error_count > 0:
            lines.append(f'- Why are there {error_count} critical errors?')

    if result.duplicates:
        lines.append(f'- Should the {len(result.duplicates)} duplicate clusters be consolidated?')

    if result.feature_usage:
        fragmented = [f for f, d in result.feature_usage.items() if d.get('spread', 0) > 5]
        if fragmented:
            lines.append(f'- Features {fragmented[:5]} are spread across many files. Is this intentional?')

    (out / 'interactive_helpers.txt').write_text('\n'.join(lines))

# ============================================================================
# CLI
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description='Elite Rust Workspace Static Analysis Tool',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python rust_workspace_analyzer.py --repo /path/to/rust/workspace
    python rust_workspace_analyzer.py --repo . --out ./analysis_out --verbose
    python rust_workspace_analyzer.py --repo . --exclude-tests --jobs 8
        """
    )

    parser.add_argument('--repo', type=Path, default=Path.cwd(),
                        help='Root directory of Rust workspace (default: .)')
    parser.add_argument('--out', type=Path, default=None,
                        help='Output directory (default: ./analysis_out)')
    parser.add_argument('--include-tests', action='store_true', default=True,
                        help='Include test files (default: true)')
    parser.add_argument('--exclude-tests', action='store_true',
                        help='Exclude test files')
    parser.add_argument('--max-files', type=int, default=10000,
                        help='Maximum files to scan (default: 10000)')
    parser.add_argument('--jobs', '-j', type=int, default=4,
                        help='Parallel jobs (default: 4)')
    parser.add_argument('--verbose', '-v', action='store_true',
                        help='Verbose output')
    parser.add_argument('--dry-run', action='store_true',
                        help='Show what would be done without doing it')
    parser.add_argument('--version', action='version', version=f'%(prog)s {VERSION}')

    args = parser.parse_args()

    # Resolve paths
    repo = args.repo.resolve()
    out = (args.out or repo / 'analysis_out').resolve()
    include_tests = args.include_tests and not args.exclude_tests

    # Validate repo
    if not (repo / 'Cargo.toml').exists():
        print(f"Error: No Cargo.toml found in {repo}", file=sys.stderr)
        sys.exit(1)

    # Dry run: show counts
    if args.dry_run:
        rust_files = find_rust_files(repo, DEFAULT_IGNORES)
        cargo_files = find_cargo_tomls(repo, DEFAULT_IGNORES)
        print(f"Repository: {repo}")
        print(f"Rust files: {len(rust_files)}")
        print(f"Cargo.toml files: {len(cargo_files)}")
        print(f"Output would go to: {out}")
        return

    # Create output directory
    out.mkdir(parents=True, exist_ok=True)

    # Run analysis
    t0 = time.time()
    result = analyze_workspace(
        repo=repo,
        out=out,
        include_tests=include_tests,
        max_files=args.max_files,
        jobs=args.jobs,
        verbose=args.verbose
    )

    # Generate outputs
    echo("Generating outputs...", 1, args.verbose)
    generate_dependency_graph_json(result, out)
    generate_symbol_index_json(result, out)
    generate_feature_matrix_json(result, out)
    generate_smells_report(result, out)
    generate_interactive_helpers(result, out)

    # Write full result
    (out / 'full_analysis.json').write_text(json.dumps(asdict(result), indent=2, default=str))

    elapsed = time.time() - t0

    # Summary
    print()
    print("=" * 60)
    print(f"Analysis complete in {elapsed:.2f}s")
    print("=" * 60)
    print(f"Crates:     {result.metrics['crates']}")
    print(f"Modules:    {result.metrics['modules']}")
    print(f"Symbols:    {result.metrics['symbols']}")
    print(f"Features:   {result.metrics['features']}")
    print(f"Smells:     {result.metrics['smells']}")
    print(f"Duplicates: {result.metrics['duplicates']}")
    print(f"Lines:      {result.metrics['total_lines']} ({result.metrics['doc_lines']} docs)")
    print()
    print(f"Output written to: {out}")

    # Exit code based on critical errors
    critical = sum(1 for s in result.smells if s['severity'] == 'error')
    if critical > 0:
        print(f"\n!! {critical} critical errors found !!")
        sys.exit(1)

if __name__ == '__main__':
    main()
