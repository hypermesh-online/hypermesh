// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// sync-status.ts - Reads crate-status.toml files and generates website data
// Usage: node --experimental-strip-types scripts/sync-status.ts

import { readFileSync, writeFileSync, readdirSync, statSync, mkdirSync, existsSync } from "fs";
import { join, resolve } from "path";
import { execSync } from "child_process";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface CrateStatus {
  id: string;
  name: string;
  description: string;
  phase: "planning" | "alpha" | "beta" | "stable";
  features: {
    working: string[];
    inDevelopment: string[];
    planned: string[];
  };
  completion: number;
}

interface CrateStats {
  id: string;
  files: number;
  linesOfCode: number;
  testCount: number;
}

// ---------------------------------------------------------------------------
// Simple TOML Parser (handles flat sections, strings, and string arrays)
// ---------------------------------------------------------------------------

interface TomlData {
  [section: string]: { [key: string]: string | string[] };
}

function parseToml(content: string): TomlData {
  const result: TomlData = {};
  let currentSection = "";

  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trim();

    // Skip empty lines and comments
    if (trimmed === "" || trimmed.startsWith("#")) {
      continue;
    }

    // Section header: [section] or [section.subsection]
    const sectionMatch = trimmed.match(/^\[([^\]]+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      if (!result[currentSection]) {
        result[currentSection] = {};
      }
      continue;
    }

    // Key = value assignment
    const kvMatch = trimmed.match(/^(\w+)\s*=\s*(.+)$/);
    if (!kvMatch) {
      continue;
    }

    const key = kvMatch[1];
    let valueStr = kvMatch[2].trim();

    // Ensure section exists
    if (!result[currentSection]) {
      result[currentSection] = {};
    }

    // String value: "something"
    const stringMatch = valueStr.match(/^"([^"]*)"$/);
    if (stringMatch) {
      result[currentSection][key] = stringMatch[1];
      continue;
    }

    // Single-line array: ["a", "b", "c"]
    if (valueStr.startsWith("[") && valueStr.endsWith("]")) {
      result[currentSection][key] = parseTomlArray(valueStr);
      continue;
    }

    // Multi-line array: starts with [ but doesn't end with ]
    if (valueStr.startsWith("[") && !valueStr.endsWith("]")) {
      let arrayContent = valueStr;
      while (i + 1 < lines.length) {
        i++;
        const nextLine = lines[i].trim();
        arrayContent += " " + nextLine;
        if (nextLine.endsWith("]")) {
          break;
        }
      }
      result[currentSection][key] = parseTomlArray(arrayContent);
      continue;
    }

    // Bare value (unquoted string or number)
    result[currentSection][key] = valueStr;
  }

  return result;
}

function parseTomlArray(arrayStr: string): string[] {
  // Remove outer brackets
  const inner = arrayStr.slice(1, -1).trim();
  if (inner === "") {
    return [];
  }

  const items: string[] = [];
  let current = "";
  let inQuotes = false;

  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];

    if (ch === '"' && (i === 0 || inner[i - 1] !== "\\")) {
      inQuotes = !inQuotes;
      continue;
    }

    if (ch === "," && !inQuotes) {
      const val = current.trim();
      if (val !== "") {
        items.push(val);
      }
      current = "";
      continue;
    }

    if (inQuotes) {
      current += ch;
    }
  }

  // Last item
  const lastVal = current.trim();
  if (lastVal !== "") {
    items.push(lastVal);
  }

  return items;
}

// ---------------------------------------------------------------------------
// Project Type Detection
// ---------------------------------------------------------------------------

type ProjectType = "rust" | "typescript";

function detectProjectType(crateDir: string): { type: ProjectType; srcDir: string } | null {
  // Rust crate: has src/ directory
  const rustSrc = join(crateDir, "src");
  if (existsSync(rustSrc)) {
    return { type: "rust", srcDir: rustSrc };
  }

  // TypeScript/JS project: has a subdirectory containing package.json
  // (e.g., ui/frontend/package.json)
  try {
    const entries = readdirSync(crateDir);
    for (const entry of entries) {
      const subDir = join(crateDir, entry);
      if (entry === "node_modules" || entry.startsWith(".")) continue;
      try {
        const stat = statSync(subDir);
        if (stat.isDirectory() && existsSync(join(subDir, "package.json"))) {
          return { type: "typescript", srcDir: subDir };
        }
      } catch {
        // Skip inaccessible entries
      }
    }
  } catch {
    // Skip inaccessible directories
  }

  return null;
}

// ---------------------------------------------------------------------------
// Code Metrics Collection - Rust
// ---------------------------------------------------------------------------

function countRsFiles(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    const output = execSync(
      `find "${srcDir}" -name "*.rs" | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

function countRsLinesOfCode(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    const output = execSync(
      `find "${srcDir}" -name "*.rs" -exec cat {} + 2>/dev/null | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

function countRsTests(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    const output = execSync(
      `grep -r "#\\[test\\]" "${srcDir}" 2>/dev/null | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Code Metrics Collection - TypeScript/JavaScript
// ---------------------------------------------------------------------------

function countTsFiles(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    const output = execSync(
      `find "${srcDir}" -not -path "*/node_modules/*" \\( -name "*.ts" -o -name "*.tsx" \\) | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

function countTsLinesOfCode(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    const output = execSync(
      `find "${srcDir}" -not -path "*/node_modules/*" \\( -name "*.ts" -o -name "*.tsx" \\) -exec cat {} + 2>/dev/null | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

function countTsTests(srcDir: string): number {
  if (!existsSync(srcDir)) {
    return 0;
  }
  try {
    // Count test files: *.test.ts(x), *.spec.ts(x), and files inside __tests__/
    const output = execSync(
      `find "${srcDir}" -not -path "*/node_modules/*" \\( -name "*.test.ts" -o -name "*.test.tsx" -o -name "*.spec.ts" -o -name "*.spec.tsx" \\) | wc -l`,
      { encoding: "utf-8" }
    );
    return parseInt(output.trim(), 10) || 0;
  } catch {
    return 0;
  }
}

// ---------------------------------------------------------------------------
// Unified Stats Collection
// ---------------------------------------------------------------------------

function collectCrateStats(crateDir: string, crateId: string): CrateStats {
  const project = detectProjectType(crateDir);
  if (!project) {
    return { id: crateId, files: 0, linesOfCode: 0, testCount: 0 };
  }

  if (project.type === "typescript") {
    return {
      id: crateId,
      files: countTsFiles(project.srcDir),
      linesOfCode: countTsLinesOfCode(project.srcDir),
      testCount: countTsTests(project.srcDir),
    };
  }

  return {
    id: crateId,
    files: countRsFiles(project.srcDir),
    linesOfCode: countRsLinesOfCode(project.srcDir),
    testCount: countRsTests(project.srcDir),
  };
}

// ---------------------------------------------------------------------------
// TOML to CrateStatus conversion
// ---------------------------------------------------------------------------

const VALID_PHASES = new Set(["planning", "alpha", "beta", "stable"]);

function tomlToCrateStatus(data: TomlData, filePath: string): CrateStatus | null {
  const crate = data["crate"];
  if (!crate) {
    console.error(`[ERROR] Missing [crate] section in ${filePath}`);
    return null;
  }

  const id = crate["id"];
  const name = crate["name"];
  const description = crate["description"];
  const phase = crate["phase"];

  if (typeof id !== "string" || typeof name !== "string") {
    console.error(`[ERROR] Missing required crate.id or crate.name in ${filePath}`);
    return null;
  }

  const resolvedPhase = (typeof phase === "string" && VALID_PHASES.has(phase))
    ? phase as CrateStatus["phase"]
    : "planning";

  const working = toStringArray(data["features.working"]?.["items"]);
  const inDevelopment = toStringArray(data["features.in_development"]?.["items"]);
  const planned = toStringArray(data["features.planned"]?.["items"]);

  const total = working.length + inDevelopment.length + planned.length;
  const completion = total > 0 ? Math.round((working.length / total) * 100) : 0;

  return {
    id,
    name,
    description: typeof description === "string" ? description : "",
    phase: resolvedPhase,
    features: { working, inDevelopment, planned },
    completion,
  };
}

function toStringArray(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.filter((v): v is string => typeof v === "string");
  }
  return [];
}

// ---------------------------------------------------------------------------
// Output generation
// ---------------------------------------------------------------------------

function generateStatusTs(statuses: CrateStatus[]): string {
  const lines: string[] = [
    "// AUTO-GENERATED by scripts/sync-status.ts",
    "// DO NOT EDIT - update crate-status.toml files instead",
    "",
    'export interface CrateStatus {',
    '  id: string;',
    '  name: string;',
    '  description: string;',
    '  phase: "planning" | "alpha" | "beta" | "stable";',
    '  features: {',
    '    working: string[];',
    '    inDevelopment: string[];',
    '    planned: string[];',
    '  };',
    '  completion: number;',
    '}',
    "",
    `export const crateStatuses: CrateStatus[] = ${formatJson(statuses)};`,
    "",
  ];
  return lines.join("\n");
}

function generateStatsTs(stats: CrateStats[]): string {
  const totalFiles = stats.reduce((sum, s) => sum + s.files, 0);
  const totalLines = stats.reduce((sum, s) => sum + s.linesOfCode, 0);
  const totalTests = stats.reduce((sum, s) => sum + s.testCount, 0);

  const totalStats = {
    totalFiles,
    totalLines,
    totalTests,
    crateCount: stats.length,
  };

  const lines: string[] = [
    "// AUTO-GENERATED by scripts/sync-status.ts",
    "// DO NOT EDIT - derived from codebase metrics",
    "",
    "export interface CrateStats {",
    "  id: string;",
    "  files: number;",
    "  linesOfCode: number;",
    "  testCount: number;",
    "}",
    "",
    `export const crateStats: CrateStats[] = ${formatJson(stats)};`,
    "",
    `export const totalStats = ${formatJson(totalStats)};`,
    "",
  ];
  return lines.join("\n");
}

function formatJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

// ---------------------------------------------------------------------------
// Roadmap validation (optional)
// ---------------------------------------------------------------------------

function validateRoadmapReferences(
  outputDir: string,
  validIds: Set<string>
): void {
  const roadmapPath = join(outputDir, "roadmap.ts");
  if (!existsSync(roadmapPath)) {
    return;
  }

  console.log("[INFO] Validating roadmap references...");
  try {
    const content = readFileSync(roadmapPath, "utf-8");
    // Match id: "..." patterns in the roadmap file
    const idMatches = content.matchAll(/id:\s*"([^"]+)"/g);
    for (const match of idMatches) {
      const refId = match[1];
      if (!validIds.has(refId)) {
        console.warn(`[WARN] Roadmap references unknown crate ID: "${refId}"`);
      }
    }
  } catch (err) {
    console.warn(`[WARN] Could not read roadmap.ts for validation: ${err}`);
  }
}

// ---------------------------------------------------------------------------
// Directory scanning
// ---------------------------------------------------------------------------

function findCrateDirectories(coreDir: string): string[] {
  const entries = readdirSync(coreDir);
  const dirs: string[] = [];

  for (const entry of entries) {
    const fullPath = join(coreDir, entry);
    try {
      const stat = statSync(fullPath);
      if (stat.isDirectory() && !entry.startsWith(".") && entry !== "node_modules") {
        dirs.push(fullPath);
      }
    } catch {
      // Skip inaccessible directories
    }
  }

  // Also scan sibling directories (outside core workspace)
  const parentDir = resolve(coreDir, "..");
  const siblingNames = ["engauge"];
  for (const name of siblingNames) {
    const siblingPath = join(parentDir, name);
    if (existsSync(siblingPath)) {
      try {
        const stat = statSync(siblingPath);
        if (stat.isDirectory()) {
          dirs.push(siblingPath);
        }
      } catch {
        // Skip inaccessible siblings
      }
    }
  }

  return dirs.sort();
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): void {
  const coreDir = resolve(join(import.meta.dirname ?? ".", ".."));
  const outputDir = join(coreDir, "scripts", "output");

  console.log(`[INFO] Core directory: ${coreDir}`);
  console.log(`[INFO] Output directory: ${outputDir}`);

  // Ensure output directory exists
  if (!existsSync(outputDir)) {
    mkdirSync(outputDir, { recursive: true });
  }

  // No timestamp in output - prevents false-positive staleness in pre-push hook
  const crateDirs = findCrateDirectories(coreDir);
  const statuses: CrateStatus[] = [];
  const stats: CrateStats[] = [];
  let hasErrors = false;
  let tomlCount = 0;

  for (const crateDir of crateDirs) {
    const dirName = crateDir.split("/").pop() ?? "";
    const tomlPath = join(crateDir, "crate-status.toml");

    // Skip directories that are not a recognized project (no src/ or package.json)
    const project = detectProjectType(crateDir);
    if (!project) {
      continue;
    }

    // Try to read crate-status.toml
    if (existsSync(tomlPath)) {
      tomlCount++;
      try {
        const content = readFileSync(tomlPath, "utf-8");
        const data = parseToml(content);
        const status = tomlToCrateStatus(data, tomlPath);

        if (status) {
          statuses.push(status);
          const crateMetrics = collectCrateStats(crateDir, status.id);
          stats.push(crateMetrics);
        } else {
          hasErrors = true;
        }
      } catch (err) {
        console.error(`[ERROR] Failed to parse ${tomlPath}: ${err}`);
        hasErrors = true;
      }
    } else {
      console.warn(`[WARN] No crate-status.toml found in ${dirName}/`);

      // Still collect code stats using the directory name as ID
      const crateMetrics = collectCrateStats(crateDir, dirName);
      if (crateMetrics.files > 0) {
        stats.push(crateMetrics);
      }
    }
  }

  console.log(`[INFO] Found ${tomlCount} crate-status.toml files`);
  console.log(`[INFO] Parsed ${statuses.length} crate statuses`);
  console.log(`[INFO] Collected stats for ${stats.length} crates`);

  // Sort by id for deterministic output
  statuses.sort((a, b) => a.id.localeCompare(b.id));
  stats.sort((a, b) => a.id.localeCompare(b.id));

  // Write output files
  const statusPath = join(outputDir, "status.ts");
  const statsPath = join(outputDir, "stats.ts");

  writeFileSync(statusPath, generateStatusTs(statuses), "utf-8");
  console.log(`[INFO] Wrote ${statusPath}`);

  writeFileSync(statsPath, generateStatsTs(stats), "utf-8");
  console.log(`[INFO] Wrote ${statsPath}`);

  // Validate roadmap references
  const validIds = new Set(statuses.map((s) => s.id));
  validateRoadmapReferences(outputDir, validIds);

  // Summary
  const totalFiles = stats.reduce((sum, s) => sum + s.files, 0);
  const totalLines = stats.reduce((sum, s) => sum + s.linesOfCode, 0);
  const totalTests = stats.reduce((sum, s) => sum + s.testCount, 0);

  console.log("");
  console.log("--- Summary ---");
  console.log(`Crates with status: ${statuses.length}`);
  console.log(`Crates with stats:  ${stats.length}`);
  console.log(`Total source files: ${totalFiles}`);
  console.log(`Total lines of code: ${totalLines}`);
  console.log(`Total tests:        ${totalTests}`);

  if (hasErrors) {
    console.error("\n[ERROR] Some crate-status.toml files had errors (see above)");
    process.exit(1);
  }

  process.exit(0);
}

main();
