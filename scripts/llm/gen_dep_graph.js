#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const outDir = path.join(root, "docs/graphs");
const outPath = path.join(outDir, "deps.json");

const metadataRaw = execSync("cargo metadata --format-version=1 --no-deps", {
  cwd: root,
  stdio: ["ignore", "pipe", "inherit"],
  maxBuffer: 50 * 1024 * 1024,
}).toString("utf8");

const metadata = JSON.parse(metadataRaw);
const workspaceIds = new Set(metadata.workspace_members);
const packages = metadata.packages.filter((p) => workspaceIds.has(p.id));

const idToPkg = new Map(packages.map((p) => [p.id, p]));
const nodes = packages.map((p) => ({
  id: p.id,
  name: p.name,
  manifest_path: p.manifest_path,
}));

const edges = [];
for (const pkg of packages) {
  for (const dep of pkg.dependencies) {
    const target = packages.find((p) => p.name === dep.name);
    if (!target) continue;
    edges.push({
      from: pkg.name,
      to: target.name,
      kind: dep.kind || "normal",
    });
  }
}

const payload = {
  generated_at: new Date().toISOString(),
  nodes,
  edges,
};

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(payload, null, 2), "utf8");
console.log(`Wrote ${outPath}`);
