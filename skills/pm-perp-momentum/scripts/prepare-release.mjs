import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const projectRoot = process.cwd();
const releaseRoot = join(projectRoot, "release", "pm-perp-momentum");

const includePaths = [
  ".claude-plugin",
  "LICENSE",
  "OPERATOR_RUNBOOK.md",
  "README.md",
  "SKILL.md",
  "SUMMARY.md",
  "eslint.config.mjs",
  "package-lock.json",
  "package.json",
  "plugin.yaml",
  "scripts",
  "src",
  "tests",
  "tsconfig.json"
];

if (existsSync(releaseRoot)) {
  rmSync(releaseRoot, { recursive: true, force: true });
}
mkdirSync(releaseRoot, { recursive: true });

for (const relPath of includePaths) {
  const sourcePath = join(projectRoot, relPath);
  const targetPath = join(releaseRoot, relPath);
  cpSync(sourcePath, targetPath, { recursive: true });
}

const strictCheck = spawnSync("node", ["scripts/submission-check.mjs", "--strict"], {
  cwd: releaseRoot,
  stdio: "inherit"
});
if (strictCheck.status !== 0) {
  process.exit(strictCheck.status ?? 1);
}

process.stdout.write(`release bundle prepared: ${releaseRoot}\n`);
