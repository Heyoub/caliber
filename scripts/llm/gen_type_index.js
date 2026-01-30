#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const { execSync } = require('node:child_process');

const root = path.resolve(__dirname, '../..');
const outDir = path.join(root, 'docs/graphs');
const outPath = path.join(outDir, 'type-signatures.json');

const files = execSync("git ls-files '*.rs'", {
  cwd: root,
  stdio: ['ignore', 'pipe', 'inherit'],
})
  .toString('utf8')
  .split('\n')
  .filter((p) => p.includes('/src/') && p.endsWith('.rs'));

const patterns = [
  { kind: 'struct', re: /^\s*pub\s+struct\s+([A-Za-z0-9_]+)/ },
  { kind: 'enum', re: /^\s*pub\s+enum\s+([A-Za-z0-9_]+)/ },
  { kind: 'trait', re: /^\s*pub\s+trait\s+([A-Za-z0-9_]+)/ },
  { kind: 'type', re: /^\s*pub\s+type\s+([A-Za-z0-9_]+)\s*=\s*(.+);/ },
  { kind: 'fn', re: /^\s*pub\s+fn\s+([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*(->\s*[^ {]+)?/ },
];

function modulePath(filePath) {
  const parts = filePath.split(path.sep);
  const crateDir = parts[0];
  const crateName = crateDir.replace(/-/g, '_');
  const srcIndex = parts.indexOf('src');
  if (srcIndex === -1) return crateName;
  const modParts = parts.slice(srcIndex + 1).map((p) => p.replace(/\.rs$/, ''));
  if (modParts[modParts.length - 1] === 'mod') modParts.pop();
  if (modParts[modParts.length - 1] === 'lib') modParts.pop();
  return [crateName, ...modParts].filter(Boolean).join('::');
}

const entries = [];
for (const relPath of files) {
  const absPath = path.join(root, relPath);
  const content = fs.readFileSync(absPath, 'utf8');
  const lines = content.split('\n');
  const modPath = modulePath(relPath);

  lines.forEach((line, idx) => {
    for (const { kind, re } of patterns) {
      const match = line.match(re);
      if (!match) continue;
      const name = match[1];
      const signature = line.trim();
      entries.push({
        kind,
        name,
        signature,
        module: modPath,
        file: relPath,
        line: idx + 1,
      });
      break;
    }
  });
}

const payload = {
  generated_at: new Date().toISOString(),
  entries,
};

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(payload, null, 2), 'utf8');
