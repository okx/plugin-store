import { existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const strictMode = process.argv.includes("--strict");
const requiredFiles = [
  "plugin.yaml",
  "SKILL.md",
  "SUMMARY.md",
  "README.md",
  "LICENSE",
  ".claude-plugin/plugin.json"
];

const disallowedTopLevelDirs = ["node_modules", "dist"];
const disallowedExactPaths = new Set(["fixtures/proof-live-csv"]);

function walkFiles(baseDir, relativeDir = "") {
  const currentDir = join(baseDir, relativeDir);
  const entries = readdirSync(currentDir, { withFileTypes: true });
  const output = [];
  for (const entry of entries) {
    const relPath = relativeDir ? join(relativeDir, entry.name) : entry.name;
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "dist" || entry.name === ".git") {
        continue;
      }
      output.push(...walkFiles(baseDir, relPath));
      continue;
    }
    output.push(relPath);
  }
  return output;
}

function fail(message) {
  process.stderr.write(`submission-check failed: ${message}\n`);
  process.exit(1);
}

for (const file of requiredFiles) {
  if (!existsSync(join(root, file))) {
    fail(`missing required file: ${file}`);
  }
}

for (const dir of disallowedTopLevelDirs) {
  const target = join(root, dir);
  if (existsSync(target) && statSync(target).isDirectory()) {
    if (strictMode) {
      fail(`remove '${dir}/' before submission packaging`);
    }
    process.stdout.write(
      `submission-check warning: '${dir}/' exists (allowed in local dev, remove before final packaging).\n`
    );
  }
}

const files = walkFiles(root).map((item) => item.replace(/\\/g, "/"));
const noisyArtifacts = files.filter(
  (name) =>
    name.endsWith(".sqlite") ||
    name.endsWith(".sqlite-shm") ||
    name.endsWith(".sqlite-wal")
);
if (noisyArtifacts.length > 0) {
  if (strictMode) {
    fail(`remove local sqlite artifacts: ${noisyArtifacts.join(", ")}`);
  }
  process.stdout.write(
    `submission-check warning: local sqlite artifacts found: ${noisyArtifacts.join(", ")}\n`
  );
}

const noisyFixtureArtifacts = files.filter(
  (name) =>
    name.startsWith("fixtures/") &&
    (name.endsWith(".json") || disallowedExactPaths.has(name))
);
if (noisyFixtureArtifacts.length > 0) {
  if (strictMode) {
    fail(
      `remove fixture artifacts before submission: ${noisyFixtureArtifacts.join(", ")}`
    );
  }
  process.stdout.write(
    `submission-check warning: fixture artifacts found: ${noisyFixtureArtifacts.join(", ")}\n`
  );
}

process.stdout.write(
  `submission-check passed${strictMode ? " (strict)" : " (dev mode)"}.\n`
);
