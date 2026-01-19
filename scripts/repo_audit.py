#!/usr/bin/env python3
"""
repo_audit.py - Repo Audit (Rust-first, generalizable)

Generates _depg/ folder with:
- dependency_graph.md      # Human-readable dependency docs
- dependency_graph.json    # Machine-parseable full AST/deps
- dependency_graph.mermaid # Visual flowchart
- dependency_graph.ascii   # Terminal-friendly view
- unused_audit.json        # All "unused" items with context
- ai_smells.json           # Phantom imports, stub logic, TODO density
- type_index.json          # All types/traits/structs with locations

Usage:
    python scripts/repo_audit.py                   # Rust scan, output to _depg/
    python scripts/repo_audit.py --json            # JSON only to stdout
    python scripts/repo_audit.py --generic-smells  # Scan non-Rust TODO/FIXME
    python scripts/repo_audit.py --extensions rs   # Override Rust extensions
    python scripts/repo_audit.py --ignore-dirs .git,vendor

Requirements: Python 3.10+ (stdlib only)
"""

from __future__ import annotations
import argparse
import json
import os
import re
import sys
import time
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional, Any


# ============================================================================
# CONFIGURATION
# ============================================================================

DEFAULT_RUST_EXTENSIONS = {'.rs'}
DEFAULT_GENERIC_EXTENSIONS = {
    '.md', '.txt', '.py', '.js', '.ts', '.tsx', '.jsx',
    '.toml', '.yaml', '.yml', '.json', '.sql', '.sh', '.ps1',
    '.rb', '.go', '.java', '.cs', '.c', '.cpp', '.h', '.hpp',
}
CARGO_FILE = 'Cargo.toml'
DEFAULT_IGNORE_DIRS = {'target', 'node_modules', '.git', 'dist', '.next', '_depg'}

# Regex patterns for Rust parsing
PATTERNS = {
    'mod_decl': re.compile(r'^\s*(?:pub\s+)?mod\s+(\w+)\s*[;{]', re.MULTILINE),
    'use_stmt': re.compile(r'^\s*(?:pub\s+)?use\s+([^;]+);', re.MULTILINE),
    'struct_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)', re.MULTILINE),
    'enum_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+(\w+)', re.MULTILINE),
    'trait_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+(\w+)', re.MULTILINE),
    'impl_block': re.compile(r'^\s*impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)', re.MULTILINE),
    'fn_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)', re.MULTILINE),
    'type_alias': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+(\w+)', re.MULTILINE),
    'const_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+(\w+)', re.MULTILINE),
    'static_def': re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?static\s+(\w+)', re.MULTILINE),
    'todo': re.compile(r'(?:TODO|FIXME|XXX|HACK|WARN)', re.IGNORECASE),
    'unimplemented': re.compile(r'(?:unimplemented!|todo!|panic!\s*\(\s*"(?:not implemented|TODO)")', re.IGNORECASE),
}

GENERIC_TODO_PATTERN = re.compile(r'(?:TODO|FIXME|XXX|HACK)', re.IGNORECASE)


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class TypeDef:
    """A type definition (struct, enum, trait, type alias)"""
    name: str
    kind: str  # 'struct', 'enum', 'trait', 'type_alias', 'const', 'static'
    file: str
    line: int
    is_pub: bool = False

@dataclass
class FunctionDef:
    """A function definition"""
    name: str
    file: str
    line: int
    is_pub: bool = False
    is_async: bool = False

@dataclass
class UseStatement:
    """A use/import statement"""
    raw: str
    path: str
    imported_items: list[str]
    file: str
    line: int
    is_pub: bool = False

@dataclass
class ImplBlock:
    """An impl block"""
    trait_name: Optional[str]  # None if inherent impl
    target_type: str
    file: str
    line: int

@dataclass
class ModuleInfo:
    """Information about a Rust module"""
    name: str
    path: str
    file: Optional[str]  # mod.rs or <name>.rs
    is_pub: bool = False
    submodules: list[str] = field(default_factory=list)
    types: list[TypeDef] = field(default_factory=list)
    functions: list[FunctionDef] = field(default_factory=list)
    uses: list[UseStatement] = field(default_factory=list)
    impls: list[ImplBlock] = field(default_factory=list)

@dataclass
class CrateInfo:
    """Information about a Cargo crate"""
    name: str
    path: Path
    cargo_toml: dict
    dependencies: list[str] = field(default_factory=list)
    dev_dependencies: list[str] = field(default_factory=list)
    modules: dict[str, ModuleInfo] = field(default_factory=dict)

@dataclass
class AISmell:
    """A detected AI code smell"""
    kind: str  # 'phantom_import', 'stub_logic', 'duplicate_def', 'wrong_path', 'todo'
    severity: str  # 'error', 'warning', 'info'
    file: str
    line: int
    message: str
    context: Optional[str] = None
    suggestion: Optional[str] = None


# ============================================================================
# PARSING FUNCTIONS
# ============================================================================

def parse_extension_list(value: Optional[str], default: set[str]) -> set[str]:
    """Parse a comma-separated extension list into a normalized set"""
    if not value:
        return set(default)
    items = [item.strip() for item in value.split(',') if item.strip()]
    normalized = set()
    for item in items:
        normalized.add(item if item.startswith('.') else f'.{item}')
    return normalized


def parse_ignore_dirs(value: Optional[str]) -> set[str]:
    """Parse comma-separated ignore dirs"""
    if not value:
        return set()
    return {item.strip() for item in value.split(',') if item.strip()}


def is_ignored(path: Path, ignore_dirs: set[str]) -> bool:
    return any(part in ignore_dirs for part in path.parts)


def iter_files(root: Path, extensions: set[str], ignore_dirs: set[str]):
    for file_path in root.rglob('*'):
        if not file_path.is_file():
            continue
        if extensions and file_path.suffix not in extensions:
            continue
        if is_ignored(file_path, ignore_dirs):
            continue
        yield file_path

def echo(msg: str, indent: int = 0) -> None:
    """Print with optional indentation"""
    prefix = "  " * indent
    print(f"{prefix}→ {msg}")


def parse_cargo_toml(path: Path) -> dict:
    """Parse a Cargo.toml file (simple TOML parser, stdlib only)"""
    content = path.read_text(encoding='utf-8')
    result = {'package': {}, 'dependencies': {}, 'dev-dependencies': {}, 'features': {}}

    current_section = None
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue

        # Section header
        if line.startswith('['):
            section = line.strip('[]').strip()
            current_section = section
            if section not in result:
                result[section] = {}
            continue

        # Key-value pair
        if '=' in line and current_section:
            key, _, value = line.partition('=')
            key = key.strip()
            value = value.strip().strip('"').strip("'")

            # Handle table subsections like [dependencies.foo]
            if '.' in current_section:
                parts = current_section.split('.', 1)
                if parts[0] not in result:
                    result[parts[0]] = {}
                if parts[1] not in result[parts[0]]:
                    result[parts[0]][parts[1]] = {}
                result[parts[0]][parts[1]][key] = value
            else:
                result[current_section][key] = value

    return result


def parse_rust_file(path: Path) -> dict:
    """Parse a Rust source file and extract definitions"""
    try:
        content = path.read_text(encoding='utf-8')
    except Exception as e:
        echo(f"Warning: Could not read {path}: {e}", 1)
        return {'error': str(e)}

    lines = content.splitlines()
    result = {
        'types': [],
        'functions': [],
        'uses': [],
        'impls': [],
        'mods': [],
        'todos': [],
        'stubs': [],
    }

    # Find all matches with line numbers
    for i, line in enumerate(lines, 1):
        # Check for TODO/FIXME
        if PATTERNS['todo'].search(line):
            result['todos'].append({'line': i, 'text': line.strip()})

        # Check for unimplemented!/todo!
        if PATTERNS['unimplemented'].search(line):
            result['stubs'].append({'line': i, 'text': line.strip()})

    # Use regex on full content for multi-line awareness
    for match in PATTERNS['mod_decl'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['mods'].append({
            'name': match.group(1),
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['use_stmt'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        raw = match.group(1).strip()
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]

        # Parse the use path
        items = parse_use_statement(raw)
        result['uses'].append({
            'raw': raw,
            'path': items['path'],
            'items': items['items'],
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['struct_def'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['types'].append({
            'name': match.group(1),
            'kind': 'struct',
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['enum_def'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['types'].append({
            'name': match.group(1),
            'kind': 'enum',
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['trait_def'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['types'].append({
            'name': match.group(1),
            'kind': 'trait',
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['type_alias'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['types'].append({
            'name': match.group(1),
            'kind': 'type_alias',
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['const_def'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        is_pub = 'pub' in content[max(0, match.start()-10):match.start()]
        result['types'].append({
            'name': match.group(1),
            'kind': 'const',
            'line': line_num,
            'is_pub': is_pub
        })

    for match in PATTERNS['fn_def'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        prefix = content[max(0, match.start()-20):match.start()]
        is_pub = 'pub' in prefix
        is_async = 'async' in prefix
        result['functions'].append({
            'name': match.group(1),
            'line': line_num,
            'is_pub': is_pub,
            'is_async': is_async
        })

    for match in PATTERNS['impl_block'].finditer(content):
        line_num = content[:match.start()].count('\n') + 1
        trait_name = match.group(1)  # None if inherent impl
        target_type = match.group(2)
        result['impls'].append({
            'trait': trait_name,
            'target': target_type,
            'line': line_num
        })

    return result


def parse_use_statement(raw: str) -> dict:
    """Parse a use statement into path and items"""
    # Handle braced imports: crate::foo::{Bar, Baz}
    if '{' in raw:
        path_part, _, items_part = raw.partition('{')
        path = path_part.strip().rstrip(':')
        items_str = items_part.rstrip('}').strip()
        items = [item.strip().split(' as ')[0].strip() for item in items_str.split(',') if item.strip()]
        return {'path': path, 'items': items}

    # Handle simple imports: crate::foo::Bar
    parts = raw.split('::')
    if parts:
        item = parts[-1].split(' as ')[0].strip()
        path = '::'.join(parts[:-1]) if len(parts) > 1 else ''
        return {'path': path, 'items': [item] if item != '*' else ['*']}

    return {'path': raw, 'items': []}


# ============================================================================
# ANALYSIS FUNCTIONS
# ============================================================================

def find_crates(root: Path, ignore_dirs: set[str]) -> list[Path]:
    """Find all Cargo.toml files (crate roots)"""
    crates = []
    for cargo in root.rglob(CARGO_FILE):
        if not is_ignored(cargo, ignore_dirs):
            crates.append(cargo.parent)
    return sorted(crates)


def scan_crate(crate_path: Path, ignore_dirs: set[str], rust_extensions: set[str]) -> CrateInfo:
    """Scan a crate and extract all information"""
    cargo_path = crate_path / CARGO_FILE
    cargo = parse_cargo_toml(cargo_path) if cargo_path.exists() else {}

    name = cargo.get('package', {}).get('name', crate_path.name)
    deps = list(cargo.get('dependencies', {}).keys())
    dev_deps = list(cargo.get('dev-dependencies', {}).keys())

    crate = CrateInfo(
        name=name,
        path=crate_path,
        cargo_toml=cargo,
        dependencies=deps,
        dev_dependencies=dev_deps,
    )

    # Scan all Rust files
    src_dir = crate_path / 'src'
    if src_dir.exists():
        for rs_file in iter_files(src_dir, rust_extensions, ignore_dirs):
            rel_path = rs_file.relative_to(crate_path)
            module_path = get_module_path(rel_path)

            parsed = parse_rust_file(rs_file)
            if 'error' not in parsed:
                mod_info = ModuleInfo(
                    name=module_path.split('::')[-1] if module_path else 'crate',
                    path=module_path,
                    file=str(rel_path),
                )

                for t in parsed.get('types', []):
                    mod_info.types.append(TypeDef(
                        name=t['name'],
                        kind=t['kind'],
                        file=str(rel_path),
                        line=t['line'],
                        is_pub=t.get('is_pub', False)
                    ))

                for f in parsed.get('functions', []):
                    mod_info.functions.append(FunctionDef(
                        name=f['name'],
                        file=str(rel_path),
                        line=f['line'],
                        is_pub=f.get('is_pub', False),
                        is_async=f.get('is_async', False)
                    ))

                for u in parsed.get('uses', []):
                    mod_info.uses.append(UseStatement(
                        raw=u['raw'],
                        path=u['path'],
                        imported_items=u['items'],
                        file=str(rel_path),
                        line=u['line'],
                        is_pub=u.get('is_pub', False)
                    ))

                for impl in parsed.get('impls', []):
                    mod_info.impls.append(ImplBlock(
                        trait_name=impl.get('trait'),
                        target_type=impl['target'],
                        file=str(rel_path),
                        line=impl['line']
                    ))

                crate.modules[module_path] = mod_info

    return crate


def get_module_path(rel_path: Path) -> str:
    """Convert a relative file path to a module path"""
    parts = list(rel_path.parts)

    # Remove 'src' prefix
    if parts and parts[0] == 'src':
        parts = parts[1:]

    # Handle lib.rs -> crate root
    if parts == ['lib.rs']:
        return ''

    # Handle main.rs -> crate root (for binaries)
    if parts == ['main.rs']:
        return ''

    # Handle mod.rs -> parent module
    if parts and parts[-1] == 'mod.rs':
        parts = parts[:-1]
    else:
        # Remove .rs extension
        if parts and parts[-1].endswith('.rs'):
            parts[-1] = parts[-1][:-3]

    return '::'.join(parts)


def detect_ai_smells(
    crates: list[CrateInfo],
    root: Path,
    ignore_dirs: set[str],
    rust_extensions: set[str],
) -> list[AISmell]:
    """Detect AI-generated code smells"""
    smells = []

    # Build type index for phantom import detection
    all_types: dict[str, list[tuple[str, str, int]]] = defaultdict(list)  # name -> [(crate, file, line)]

    for crate in crates:
        for mod_path, mod_info in crate.modules.items():
            for type_def in mod_info.types:
                all_types[type_def.name].append((crate.name, type_def.file, type_def.line))

    # Check each crate for smells
    for crate in crates:
        src_dir = crate.path / 'src'
        if not src_dir.exists():
            continue

        for rs_file in iter_files(src_dir, rust_extensions, ignore_dirs):
            parsed = parse_rust_file(rs_file)
            if 'error' in parsed:
                continue

            rel_path = str(rs_file.relative_to(root))

            # Check for phantom imports (imports of types that don't exist)
            for use in parsed.get('uses', []):
                for item in use['items']:
                    if item == '*' or item == 'self':
                        continue
                    # Check if this type exists anywhere
                    if item not in all_types:
                        # Could be external crate type - check if path starts with known crate
                        path = use['path']
                        if not any(path.startswith(c.name) for c in crates):
                            continue  # External crate, skip
                        smells.append(AISmell(
                            kind='phantom_import',
                            severity='error',
                            file=rel_path,
                            line=use['line'],
                            message=f"Phantom import: `{item}` from `{use['path']}` - type does not exist",
                            context=use['raw'],
                            suggestion=f"Create `{item}` or fix the import path"
                        ))

            # Check for TODO/FIXME density
            todos = parsed.get('todos', [])
            if len(todos) > 5:
                smells.append(AISmell(
                    kind='todo_density',
                    severity='warning',
                    file=rel_path,
                    line=todos[0]['line'],
                    message=f"High TODO/FIXME density: {len(todos)} items",
                    context=f"First: {todos[0]['text'][:50]}...",
                    suggestion="Review and address TODOs or remove if resolved"
                ))

            # Check for unimplemented!/todo! stubs
            stubs = parsed.get('stubs', [])
            for stub in stubs:
                smells.append(AISmell(
                    kind='stub_logic',
                    severity='warning',
                    file=rel_path,
                    line=stub['line'],
                    message="Stub logic detected (unimplemented!/todo!)",
                    context=stub['text'][:80],
                    suggestion="Implement or mark as intentionally unimplemented"
                ))

    # Check for duplicate definitions
    for type_name, locations in all_types.items():
        if len(locations) > 1:
            # Filter to same-crate duplicates (cross-crate is expected)
            by_crate = defaultdict(list)
            for crate_name, file, line in locations:
                by_crate[crate_name].append((file, line))

            for crate_name, locs in by_crate.items():
                if len(locs) > 1:
                    smells.append(AISmell(
                        kind='duplicate_def',
                        severity='error',
                        file=locs[0][0],
                        line=locs[0][1],
                        message=f"Duplicate definition of `{type_name}` in {crate_name}",
                        context=f"Also at: {', '.join(f'{f}:{l}' for f, l in locs[1:])}",
                        suggestion="Consolidate to single definition"
                    ))

    # Check for wrong path patterns (common AI mistake)
    wrong_path_patterns = [
        (r'super::jam::', 'Using super::jam:: - likely should be crate::events::'),
        (r'crate::context::', 'Using crate::context:: - check if should be crate::session::context::'),
    ]

    for crate in crates:
        src_dir = crate.path / 'src'
        if not src_dir.exists():
            continue

        for rs_file in iter_files(src_dir, rust_extensions, ignore_dirs):
            try:
                content = rs_file.read_text(encoding='utf-8')
            except:
                continue

            rel_path = str(rs_file.relative_to(root))

            for pattern, message in wrong_path_patterns:
                for match in re.finditer(pattern, content):
                    line_num = content[:match.start()].count('\n') + 1
                    smells.append(AISmell(
                        kind='wrong_path',
                        severity='warning',
                        file=rel_path,
                        line=line_num,
                        message=message,
                        context=content.splitlines()[line_num-1].strip()[:80] if line_num <= len(content.splitlines()) else '',
                        suggestion="Verify module path is correct"
                    ))

    return sorted(smells, key=lambda s: (s.file, s.line))


def detect_generic_smells(
    root: Path,
    ignore_dirs: set[str],
    extensions: set[str],
) -> list[AISmell]:
    """Detect generic TODO/FIXME smells in non-Rust files"""
    smells: list[AISmell] = []
    for file_path in iter_files(root, extensions, ignore_dirs):
        try:
            content = file_path.read_text(encoding='utf-8')
        except Exception:
            continue
        rel_path = str(file_path.relative_to(root))
        for idx, line in enumerate(content.splitlines(), start=1):
            if GENERIC_TODO_PATTERN.search(line):
                smells.append(AISmell(
                    kind='todo',
                    severity='info',
                    file=rel_path,
                    line=idx,
                    message="TODO/FIXME marker",
                    context=line.strip()[:120],
                    suggestion="Address or remove TODO/FIXME"
                ))
    return smells


def build_type_index(crates: list[CrateInfo]) -> dict:
    """Build a searchable index of all types"""
    index = {}

    for crate in crates:
        for mod_path, mod_info in crate.modules.items():
            for type_def in mod_info.types:
                full_path = f"{crate.name}::{mod_path}::{type_def.name}" if mod_path else f"{crate.name}::{type_def.name}"
                index[full_path] = {
                    'name': type_def.name,
                    'kind': type_def.kind,
                    'crate': crate.name,
                    'module': mod_path,
                    'file': type_def.file,
                    'line': type_def.line,
                    'is_pub': type_def.is_pub,
                }

    return dict(sorted(index.items()))


# ============================================================================
# OUTPUT GENERATORS
# ============================================================================

def generate_markdown(crates: list[CrateInfo], smells: list[AISmell]) -> str:
    """Generate dependency_graph.md"""
    lines = [
        "# Dependency Graph",
        "",
        f"Generated: {time.strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        "## Crate Overview",
        "",
        "| Crate | Dependencies | Modules | Types | Functions |",
        "|-------|--------------|---------|-------|-----------|",
    ]

    for crate in crates:
        num_types = sum(len(m.types) for m in crate.modules.values())
        num_funcs = sum(len(m.functions) for m in crate.modules.values())
        deps = ', '.join(crate.dependencies[:3]) + ('...' if len(crate.dependencies) > 3 else '')
        lines.append(f"| {crate.name} | {deps or 'none'} | {len(crate.modules)} | {num_types} | {num_funcs} |")

    lines.extend(["", "## Dependency Chain", ""])

    # Build dependency order
    for crate in crates:
        if crate.dependencies:
            local_deps = [d for d in crate.dependencies if any(c.name == d for c in crates)]
            if local_deps:
                lines.append(f"- **{crate.name}** depends on: {', '.join(local_deps)}")

    lines.extend(["", "## Module Structure", ""])

    for crate in crates:
        lines.append(f"### {crate.name}")
        lines.append("")
        lines.append("```")
        for mod_path, mod_info in sorted(crate.modules.items()):
            prefix = "  " * mod_path.count('::') if mod_path else ""
            name = mod_path.split('::')[-1] if mod_path else "lib"
            types_count = len(mod_info.types)
            funcs_count = len(mod_info.functions)
            lines.append(f"{prefix}{name}/ ({types_count} types, {funcs_count} fns)")
        lines.append("```")
        lines.append("")

    if smells:
        lines.extend(["## AI Smells Detected", ""])

        by_kind = defaultdict(list)
        for smell in smells:
            by_kind[smell.kind].append(smell)

        for kind, kind_smells in sorted(by_kind.items()):
            lines.append(f"### {kind.replace('_', ' ').title()} ({len(kind_smells)})")
            lines.append("")
            for smell in kind_smells[:10]:  # Limit to first 10
                lines.append(f"- `{smell.file}:{smell.line}` - {smell.message}")
            if len(kind_smells) > 10:
                lines.append(f"- ... and {len(kind_smells) - 10} more")
            lines.append("")

    return '\n'.join(lines)


def generate_mermaid(crates: list[CrateInfo]) -> str:
    """Generate dependency_graph.mermaid"""
    lines = [
        "```mermaid",
        "graph TD",
        "    subgraph Crates",
    ]

    # Add crate nodes
    for crate in crates:
        num_types = sum(len(m.types) for m in crate.modules.values())
        lines.append(f"        {crate.name}[{crate.name}<br/>{num_types} types]")

    lines.append("    end")
    lines.append("")

    # Add dependencies
    for crate in crates:
        for dep in crate.dependencies:
            if any(c.name == dep for c in crates):
                lines.append(f"    {crate.name} --> {dep}")

    lines.append("```")
    return '\n'.join(lines)


def generate_ascii(crates: list[CrateInfo]) -> str:
    """Generate dependency_graph.ascii"""
    lines = [
        "=" * 60,
        "DEPENDENCY GRAPH (ASCII)",
        "=" * 60,
        "",
    ]

    for crate in crates:
        num_types = sum(len(m.types) for m in crate.modules.values())
        num_funcs = sum(len(m.functions) for m in crate.modules.values())

        lines.append(f"┌{'─' * 40}┐")
        lines.append(f"│ {crate.name:<38} │")
        lines.append(f"│ Types: {num_types:<5} Functions: {num_funcs:<5}      │")

        if crate.dependencies:
            local_deps = [d for d in crate.dependencies if any(c.name == d for c in crates)]
            if local_deps:
                lines.append(f"│ Depends: {', '.join(local_deps):<28} │")

        lines.append(f"└{'─' * 40}┘")

        # Show modules
        for mod_path, mod_info in sorted(crate.modules.items()):
            depth = mod_path.count('::') if mod_path else 0
            prefix = "    │" * depth + "    ├── " if depth > 0 else "    ├── "
            name = mod_path.split('::')[-1] if mod_path else "lib.rs"
            lines.append(f"{prefix}{name}")

        lines.append("")

    return '\n'.join(lines)


def generate_json(crates: list[CrateInfo], smells: list[AISmell], type_index: dict) -> dict:
    """Generate full JSON output"""
    return {
        'generated_at': time.strftime('%Y-%m-%dT%H:%M:%SZ'),
        'crates': [
            {
                'name': c.name,
                'path': str(c.path),
                'dependencies': c.dependencies,
                'dev_dependencies': c.dev_dependencies,
                'modules': {
                    path: {
                        'name': mod.name,
                        'file': mod.file,
                        'types': [asdict(t) for t in mod.types],
                        'functions': [asdict(f) for f in mod.functions],
                        'uses': [asdict(u) for u in mod.uses],
                        'impls': [asdict(i) for i in mod.impls],
                    }
                    for path, mod in c.modules.items()
                }
            }
            for c in crates
        ],
        'ai_smells': [asdict(s) for s in smells],
        'type_index': type_index,
        'stats': {
            'total_crates': len(crates),
            'total_modules': sum(len(c.modules) for c in crates),
            'total_types': sum(sum(len(m.types) for m in c.modules.values()) for c in crates),
            'total_functions': sum(sum(len(m.functions) for m in c.modules.values()) for c in crates),
            'total_smells': len(smells),
            'smells_by_kind': dict(sorted(
                {k: len([s for s in smells if s.kind == k]) for k in set(s.kind for s in smells)}.items()
            )) if smells else {},
        }
    }


def generate_unused_audit(crates: list[CrateInfo], root: Path) -> dict:
    """Generate unused_audit.json - track all imports and their usage"""
    audit = {
        'imports': [],
        'types': [],
        'functions': [],
    }

    # Collect all imports
    for crate in crates:
        for mod_path, mod_info in crate.modules.items():
            for use in mod_info.uses:
                audit['imports'].append({
                    'crate': crate.name,
                    'module': mod_path,
                    'file': use.file,
                    'line': use.line,
                    'path': use.path,
                    'items': use.imported_items,
                    'is_pub': use.is_pub,
                    'raw': use.raw,
                })

    # Collect all type definitions
    for crate in crates:
        for mod_path, mod_info in crate.modules.items():
            for t in mod_info.types:
                audit['types'].append({
                    'crate': crate.name,
                    'module': mod_path,
                    'name': t.name,
                    'kind': t.kind,
                    'file': t.file,
                    'line': t.line,
                    'is_pub': t.is_pub,
                })

    # Collect all function definitions
    for crate in crates:
        for mod_path, mod_info in crate.modules.items():
            for f in mod_info.functions:
                audit['functions'].append({
                    'crate': crate.name,
                    'module': mod_path,
                    'name': f.name,
                    'file': f.file,
                    'line': f.line,
                    'is_pub': f.is_pub,
                    'is_async': f.is_async,
                })

    return audit


# ============================================================================
# MAIN
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description='Repo audit (Rust-first, generalizable)')
    parser.add_argument('--json', action='store_true', help='Output JSON only to stdout')
    parser.add_argument('--watch', action='store_true', help='Watch mode (future)')
    parser.add_argument('--extensions', help='Rust extensions to parse (comma-separated)')
    parser.add_argument('--generic-smells', action='store_true', help='Scan non-Rust TODO/FIXME')
    parser.add_argument('--generic-extensions', help='Non-Rust extensions for generic scan')
    parser.add_argument('--ignore-dirs', help='Extra ignore dirs (comma-separated)')
    parser.add_argument('--out-dir', help='Output directory (default: _depg)')
    parser.add_argument('root', nargs='?', default='.', help='Root directory to scan')
    args = parser.parse_args()

    root = Path(args.root).resolve()
    echo(f"Scanning: {root}")

    start_time = time.time()
    ignore_dirs = DEFAULT_IGNORE_DIRS | parse_ignore_dirs(args.ignore_dirs)
    rust_extensions = parse_extension_list(args.extensions, DEFAULT_RUST_EXTENSIONS)
    generic_extensions = parse_extension_list(args.generic_extensions, DEFAULT_GENERIC_EXTENSIONS)

    # Find all crates
    echo("Finding crates...")
    crate_paths = find_crates(root, ignore_dirs)
    echo(f"Found {len(crate_paths)} crates", 1)

    # Scan each crate
    crates = []
    for crate_path in crate_paths:
        echo(f"Scanning {crate_path.name}...", 1)
        crate = scan_crate(crate_path, ignore_dirs, rust_extensions)
        crates.append(crate)
        echo(f"  {len(crate.modules)} modules, {sum(len(m.types) for m in crate.modules.values())} types", 2)

    # Detect AI smells
    echo("Detecting AI smells...")
    smells = detect_ai_smells(crates, root, ignore_dirs, rust_extensions)
    if args.generic_smells:
        smells.extend(detect_generic_smells(root, ignore_dirs, generic_extensions))
    echo(f"Found {len(smells)} smells", 1)

    # Build type index
    echo("Building type index...")
    type_index = build_type_index(crates)
    echo(f"Indexed {len(type_index)} types", 1)

    elapsed = time.time() - start_time
    echo(f"Scan complete in {elapsed:.2f}s")

    if args.json:
        # JSON only to stdout
        print(json.dumps(generate_json(crates, smells, type_index), indent=2))
        return

    # Create output directory
    output_dir = root / (args.out_dir or '_depg')
    output_dir.mkdir(exist_ok=True)
    echo(f"Writing to {output_dir}")

    # Generate all outputs
    outputs = {
        'dependency_graph.md': generate_markdown(crates, smells),
        'dependency_graph.mermaid': generate_mermaid(crates),
        'dependency_graph.ascii': generate_ascii(crates),
        'dependency_graph.json': json.dumps(generate_json(crates, smells, type_index), indent=2),
        'unused_audit.json': json.dumps(generate_unused_audit(crates, root), indent=2),
        'ai_smells.json': json.dumps([asdict(s) for s in smells], indent=2),
        'type_index.json': json.dumps(type_index, indent=2),
    }

    for filename, content in outputs.items():
        path = output_dir / filename
        path.write_text(content, encoding='utf-8')
        echo(f"Wrote {filename} ({len(content)} bytes)", 1)

    # Summary
    echo("")
    echo("=" * 50)
    echo("SUMMARY")
    echo("=" * 50)
    echo(f"Crates: {len(crates)}")
    echo(f"Modules: {sum(len(c.modules) for c in crates)}")
    echo(f"Types: {sum(sum(len(m.types) for m in c.modules.values()) for c in crates)}")
    echo(f"Functions: {sum(sum(len(m.functions) for m in c.modules.values()) for c in crates)}")
    echo(f"AI Smells: {len(smells)}")

    if smells:
        echo("")
        echo("Top smells:")
        by_kind = defaultdict(int)
        for smell in smells:
            by_kind[smell.kind] += 1
        for kind, count in sorted(by_kind.items(), key=lambda x: -x[1]):
            echo(f"  {kind}: {count}", 1)


if __name__ == '__main__':
    main()
