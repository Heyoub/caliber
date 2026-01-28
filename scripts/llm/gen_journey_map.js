#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const outDir = path.join(root, "docs/journeys");
const jsonPath = path.join(outDir, "journey-map.json");
const mdPath = path.join(outDir, "happy-path.md");

const journey = {
  generated_at: new Date().toISOString(),
  name: "critical-path",
  description: "Create trajectory → scope → artifact → close scope → complete trajectory",
  steps: [
    {
      id: "trajectory.create",
      action: "Create trajectory",
      route: { method: "POST", path: "/api/v1/trajectories" },
      handler: "caliber-api/src/routes/trajectory.rs",
      db: "DbClient::create<TrajectoryResponse>",
      tests: ["tests/e2e/critical-path.e2e.test.ts", "caliber-api/src/routes/trajectory.rs"],
    },
    {
      id: "scope.create",
      action: "Create scope",
      route: { method: "POST", path: "/api/v1/scopes" },
      handler: "caliber-api/src/routes/scope.rs",
      db: "DbClient::create<ScopeResponse>",
      tests: ["tests/e2e/critical-path.e2e.test.ts", "caliber-api/src/routes/scope.rs"],
    },
    {
      id: "artifact.create",
      action: "Create artifact",
      route: { method: "POST", path: "/api/v1/artifacts" },
      handler: "caliber-api/src/routes/artifact.rs",
      db: "DbClient::create<ArtifactResponse>",
      tests: ["tests/e2e/critical-path.e2e.test.ts", "caliber-api/src/routes/artifact.rs"],
    },
    {
      id: "scope.close",
      action: "Close scope",
      route: { method: "POST", path: "/api/v1/scopes/{id}/close" },
      handler: "caliber-api/src/routes/scope.rs",
      db: "DbClient::scope_close",
      tests: ["tests/e2e/critical-path.e2e.test.ts"],
    },
    {
      id: "trajectory.complete",
      action: "Complete trajectory",
      route: { method: "PATCH", path: "/api/v1/trajectories/{id}" },
      handler: "caliber-api/src/routes/trajectory.rs",
      db: "DbClient::update_raw<TrajectoryResponse>",
      tests: ["tests/e2e/critical-path.e2e.test.ts"],
    },
  ],
};

const md = `# Critical Path Journey\n\n${journey.description}\n\n` +
  journey.steps.map((s, i) => {
    return `${i + 1}. ${s.action}\n` +
      `   - Route: ${s.route.method} ${s.route.path}\n` +
      `   - Handler: ${s.handler}\n` +
      `   - DB: ${s.db}\n` +
      `   - Tests: ${s.tests.join(", ")}\n`;
  }).join("\n");

fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(jsonPath, JSON.stringify(journey, null, 2), "utf8");
fs.writeFileSync(mdPath, md + "\n", "utf8");

console.log(`Wrote ${jsonPath}`);
console.log(`Wrote ${mdPath}`);
