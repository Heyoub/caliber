#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");

function usage() {
  console.error("Usage: nextest_to_junit.js <input.jsonl> <junit.xml> <summary.json>");
  process.exit(1);
}

if (process.argv.length < 5) usage();

const [inputPath, junitPath, summaryPath] = process.argv.slice(2);
if (!fs.existsSync(inputPath)) {
  console.error(`Input file not found: ${inputPath}`);
  process.exit(1);
}

const lines = fs.readFileSync(inputPath, "utf8").split("\n").filter(Boolean);
const tests = new Map();
let suiteStart = null;
let suiteEnd = null;

for (const line of lines) {
  let obj;
  try {
    obj = JSON.parse(line);
  } catch {
    continue;
  }
  if (obj.type === "suite") {
    if (obj.event === "started") suiteStart = obj;
    if (obj.event === "ok" || obj.event === "failed") suiteEnd = obj;
    continue;
  }

  if (obj.type !== "test") continue;
  const name = obj.name || "<unknown>";
  const entry = tests.get(name) || {
    name,
    status: "unknown",
    time: 0,
    stdout: "",
    stderr: "",
  };

  if (obj.event === "started") {
    entry.status = entry.status === "unknown" ? "started" : entry.status;
  } else if (obj.event === "ok") {
    entry.status = "passed";
    if (typeof obj.exec_time === "number") entry.time = obj.exec_time;
  } else if (obj.event === "failed" || obj.event === "timeout") {
    entry.status = "failed";
    if (typeof obj.exec_time === "number") entry.time = obj.exec_time;
    if (obj.stdout) entry.stdout = obj.stdout;
    if (obj.stderr) entry.stderr = obj.stderr;
  } else if (obj.event === "ignored") {
    entry.status = "skipped";
  }

  tests.set(name, entry);
}

let total = 0;
let passed = 0;
let failed = 0;
let skipped = 0;
let duration = 0;

for (const t of tests.values()) {
  total += 1;
  if (t.status === "passed") passed += 1;
  else if (t.status === "failed") failed += 1;
  else if (t.status === "skipped") skipped += 1;
  duration += t.time || 0;
}

const summary = {
  total,
  passed,
  failed,
  skipped,
  duration_seconds: duration,
  started_at: suiteStart?.timestamp || null,
  finished_at: suiteEnd?.timestamp || null,
};

function xmlEscape(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

let xml = "";
xml += `<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n`;
xml += `<testsuite name=\"nextest\" tests=\"${total}\" failures=\"${failed}\" skipped=\"${skipped}\" time=\"${duration}\">\n`;

for (const t of tests.values()) {
  const time = t.time || 0;
  xml += `  <testcase name=\"${xmlEscape(t.name)}\" time=\"${time}\">\n`;
  if (t.status === "failed") {
    const details = [t.stdout, t.stderr].filter(Boolean).join("\n");
    xml += `    <failure message=\"failed\">${xmlEscape(details)}</failure>\n`;
  } else if (t.status === "skipped") {
    xml += "    <skipped />\n";
  }
  xml += "  </testcase>\n";
}

xml += "</testsuite>\n";

fs.mkdirSync(path.dirname(junitPath), { recursive: true });
fs.writeFileSync(junitPath, xml, "utf8");
fs.mkdirSync(path.dirname(summaryPath), { recursive: true });
fs.writeFileSync(summaryPath, JSON.stringify(summary, null, 2), "utf8");
