use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::plugin_yaml::{
    PluginYaml, VALID_CATEGORIES, VALID_LICENSES, VALID_MCP_TYPES, VALID_RISK_LEVELS,
};

/// A single lint finding.
#[derive(Debug)]
pub struct LintDiag {
    pub level: DiagLevel,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiagLevel {
    Error,
    Warning,
}

impl std::fmt::Display for LintDiag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = match self.level {
            DiagLevel::Error => "❌",
            DiagLevel::Warning => "⚠️ ",
        };
        write!(f, "{} [{}] {}", icon, self.code, self.message)
    }
}

/// Result of running all lint checks.
pub struct LintReport {
    pub diagnostics: Vec<LintDiag>,
    pub plugin_name: String,
}

impl LintReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagLevel::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == DiagLevel::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == DiagLevel::Warning)
            .count()
    }
}

/// Run all lint checks on a submission directory.
///
/// Expected layout:
/// ```text
/// <submission_dir>/
///   plugin.yaml
///   skills/<name>/SKILL.md
///   LICENSE (or LICENSE.md)
/// ```
pub fn lint_submission(submission_dir: &Path) -> Result<LintReport> {
    let mut diags = Vec::new();
    let mut plugin_name = String::from("<unknown>");

    // ── 1. plugin.yaml existence ──────────────────────────────────
    let yaml_path = submission_dir.join("plugin.yaml");
    if !yaml_path.exists() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E001",
            message: format!(
                "plugin.yaml not found in {}",
                submission_dir.display()
            ),
        });
        return Ok(LintReport {
            diagnostics: diags,
            plugin_name,
        });
    }

    // ── 2. Parse plugin.yaml ──────────────────────────────────────
    let yaml_content = std::fs::read_to_string(&yaml_path)
        .context("Failed to read plugin.yaml")?;

    let plugin: PluginYaml = match serde_yaml::from_str(&yaml_content) {
        Ok(p) => p,
        Err(e) => {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E002",
                message: format!("plugin.yaml parse error: {}", e),
            });
            return Ok(LintReport {
                diagnostics: diags,
                plugin_name,
            });
        }
    };
    plugin_name.clone_from(&plugin.name);

    // ── 3. Name validation ────────────────────────────────────────
    check_name(&plugin.name, &mut diags);

    // ── 4. Reserved prefix check ──────────────────────────────────
    check_reserved_prefix(&plugin.name, &mut diags);

    // ── 5. Version validation (semver) ────────────────────────────
    check_version(&plugin.version, &mut diags);

    // ── 6. Description ────────────────────────────────────────────
    check_description(&plugin.description, &mut diags);

    // ── 7. Author validation ──────────────────────────────────────
    check_author(&plugin.author, &mut diags);

    // ── 8. License validation ─────────────────────────────────────
    check_license(&plugin.license, submission_dir, &mut diags);

    // ── 9. Category validation ────────────────────────────────────
    if !VALID_CATEGORIES.contains(&plugin.category.as_str()) {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E040",
            message: format!(
                "invalid category '{}'. Valid: {}",
                plugin.category,
                VALID_CATEGORIES.join(", ")
            ),
        });
    }

    // ── 10. Components validation ─────────────────────────────────
    check_components(&plugin, submission_dir, &mut diags);

    // ── 11. Permissions validation ────────────────────────────────
    check_permissions(&plugin, &mut diags);

    // ── 12. Extra / risk_level validation ─────────────────────────
    if let Some(ref extra) = plugin.extra {
        if let Some(ref rl) = extra.risk_level {
            if !VALID_RISK_LEVELS.contains(&rl.as_str()) {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E070",
                    message: format!(
                        "invalid risk_level '{}'. Valid: {}",
                        rl,
                        VALID_RISK_LEVELS.join(", ")
                    ),
                });
            }
        }
    }

    // ── 13. File size limits ──────────────────────────────────────
    check_file_sizes(submission_dir, &mut diags);

    // ── 14. SKILL.md content validation ───────────────────────────
    check_skill_md(&plugin, submission_dir, &mut diags);

    // ── 15. PR scope: directory name matches plugin name ──────────
    check_dir_name_match(&plugin.name, submission_dir, &mut diags);

    // ── 16. onchainos API bypass detection ───────────────────────
    check_onchainos_bypass(&plugin, submission_dir, &mut diags);

    Ok(LintReport {
        diagnostics: diags,
        plugin_name,
    })
}

// ═══════════════════════════════════════════════════════════════════
//  Individual check functions
// ═══════════════════════════════════════════════════════════════════

fn check_name(name: &str, diags: &mut Vec<LintDiag>) {
    let re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").unwrap();

    if name.len() < 2 || name.len() > 40 {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E030",
            message: format!(
                "name '{}' must be 2-40 characters (got {})",
                name,
                name.len()
            ),
        });
    } else if !re.is_match(name) {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E031",
            message: format!(
                "name '{}' must be lowercase alphanumeric with hyphens only \
                 (regex: [a-z0-9][a-z0-9-]*[a-z0-9])",
                name
            ),
        });
    }

    if name.contains("--") {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E032",
            message: "name must not contain consecutive hyphens".to_string(),
        });
    }
}

fn check_reserved_prefix(name: &str, diags: &mut Vec<LintDiag>) {
    const RESERVED: &[&str] = &["okx-", "official-", "plugin-store-"];
    for prefix in RESERVED {
        if name.starts_with(prefix) {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E033",
                message: format!(
                    "name '{}' uses reserved prefix '{}' — reserved for official plugins",
                    name, prefix
                ),
            });
        }
    }
}

fn check_version(version: &str, diags: &mut Vec<LintDiag>) {
    let parts: Vec<&str> = version.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));

    if !valid {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E035",
            message: format!(
                "version '{}' is not valid semver (expected x.y.z)",
                version
            ),
        });
    }
}

fn check_description(desc: &str, diags: &mut Vec<LintDiag>) {
    if desc.trim().is_empty() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E010",
            message: "description is empty".to_string(),
        });
    } else if desc.len() > 200 {
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W010",
            message: format!(
                "description is {} chars (recommended < 200)",
                desc.len()
            ),
        });
    }
}

fn check_author(author: &super::plugin_yaml::AuthorInfo, diags: &mut Vec<LintDiag>) {
    if author.name.trim().is_empty() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E020",
            message: "author.name is empty".to_string(),
        });
    }
    if author.github.trim().is_empty() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E021",
            message: "author.github is required".to_string(),
        });
    }
}

fn check_license(license: &str, dir: &Path, diags: &mut Vec<LintDiag>) {
    if !VALID_LICENSES.contains(&license) {
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W040",
            message: format!(
                "license '{}' is not a common SPDX identifier. Known: {}",
                license,
                VALID_LICENSES.join(", ")
            ),
        });
    }

    let has_license_file = dir.join("LICENSE").exists() || dir.join("LICENSE.md").exists();
    if !has_license_file {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E041",
            message: "LICENSE file not found in submission directory".to_string(),
        });
    }
}

fn check_components(plugin: &PluginYaml, dir: &Path, diags: &mut Vec<LintDiag>) {
    let has_any = plugin.components.skill.is_some()
        || plugin.components.mcp.is_some()
        || plugin.components.binary.is_some();

    if !has_any {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E050",
            message: "at least one component (skill, mcp, or binary) must be declared"
                .to_string(),
        });
    }

    // ── Skill component ──────────────────────────────────────────
    if let Some(ref skill) = plugin.components.skill {
        if let Some(ref skill_dir) = skill.dir {
            let skill_dir_path = dir.join(skill_dir);
            if !skill_dir_path.exists() {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E051",
                    message: format!(
                        "skill dir '{}' does not exist in submission",
                        skill_dir
                    ),
                });
            } else {
                let skill_md = skill_dir_path.join("SKILL.md");
                if !skill_md.exists() {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E052",
                        message: format!("SKILL.md not found in '{}'", skill_dir),
                    });
                }
            }
        }
        if skill.dir.is_none() && skill.path.is_none() && skill.repo.is_none() {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W051",
                message:
                    "skill component has no dir, path, or repo — auto-discover will be used"
                        .to_string(),
            });
        }
    }

    // ── MCP component ────────────────────────────────────────────
    if let Some(ref mcp) = plugin.components.mcp {
        if !VALID_MCP_TYPES.contains(&mcp.mcp_type.as_str()) {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E055",
                message: format!(
                    "mcp.type '{}' invalid. Valid: {}",
                    mcp.mcp_type,
                    VALID_MCP_TYPES.join(", ")
                ),
            });
        }
        if mcp.command.trim().is_empty() {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E056",
                message: "mcp.command is empty".to_string(),
            });
        }
        // Shell injection check
        let dangerous_chars = ['|', ';', '&', '$', '`', '(', ')', '{', '}'];
        if mcp
            .command
            .contains(|c: char| dangerous_chars.contains(&c))
        {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E057",
                message: format!(
                    "mcp.command '{}' contains shell metacharacters — possible command injection",
                    mcp.command
                ),
            });
        }
        for arg in &mcp.args {
            if arg.contains(|c: char| dangerous_chars.contains(&c)) {
                diags.push(LintDiag {
                    level: DiagLevel::Warning,
                    code: "W057",
                    message: format!(
                        "mcp.args '{}' contains shell metacharacters",
                        arg
                    ),
                });
            }
        }
    }

    // ── Binary component ─────────────────────────────────────────
    if let Some(ref bin) = plugin.components.binary {
        if bin.repo.trim().is_empty() {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E060",
                message: "binary.repo is empty".to_string(),
            });
        }
        if bin.checksums_asset.is_none() {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W060",
                message:
                    "binary.checksums_asset not set — SHA256 verification will be skipped"
                        .to_string(),
            });
        }
    }
}

fn check_permissions(plugin: &PluginYaml, diags: &mut Vec<LintDiag>) {
    if plugin.permissions.is_none() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E065",
            message: "permissions field is required — declare what your plugin can do"
                .to_string(),
        });
        return;
    }

    let perms = plugin.permissions.as_ref().unwrap();

    if let Some(ref wallet) = perms.wallet {
        if wallet.send_transaction {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W065",
                message:
                    "wallet.send_transaction is true — this plugin can initiate transfers. \
                     Community plugins cannot use this permission on first submission."
                        .to_string(),
            });
        }
        if wallet.contract_call {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W066",
                message:
                    "wallet.contract_call is true — this plugin can call smart contracts. \
                     Community plugins cannot use this permission on first submission."
                        .to_string(),
            });
        }
    }

    if perms.chains.is_empty() {
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W067",
            message:
                "permissions.chains is empty — declare which chains your plugin operates on"
                    .to_string(),
        });
    }

    // Cross-check: if SKILL.md references onchainos commands not listed in permissions
    // (done in check_skill_md via check_onchainos_command_consistency)
}

fn check_file_sizes(dir: &Path, diags: &mut Vec<LintDiag>) {
    const MAX_SINGLE_FILE: u64 = 100 * 1024; // 100 KB
    const MAX_TOTAL: u64 = 1024 * 1024; // 1 MB

    let mut total = 0u64;

    if let Ok(entries) = walk_dir(dir) {
        for entry in entries {
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                total += size;

                if size > MAX_SINGLE_FILE {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E080",
                        message: format!(
                            "file '{}' is {} KB (limit: 100 KB)",
                            entry.strip_prefix(dir).unwrap_or(&entry).display(),
                            size / 1024
                        ),
                    });
                }
            }
        }
    }

    if total > MAX_TOTAL {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E081",
            message: format!(
                "total submission size is {} KB (limit: 1024 KB)",
                total / 1024
            ),
        });
    }
}

fn check_skill_md(plugin: &PluginYaml, dir: &Path, diags: &mut Vec<LintDiag>) {
    let skill_md_path = find_skill_md(plugin, dir);

    let path = match skill_md_path {
        Some(p) if p.exists() => p,
        _ => return, // Already reported in check_components
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // ── Frontmatter check ────────────────────────────────────────
    if !content.starts_with("---") {
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W090",
            message: "SKILL.md does not start with YAML frontmatter (---)"
                .to_string(),
        });
    } else {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() >= 3 {
            let frontmatter = parts[1].trim();
            match serde_yaml::from_str::<serde_yaml::Value>(frontmatter) {
                Ok(val) => {
                    if let Some(map) = val.as_mapping() {
                        for field in ["name", "description"] {
                            if !map.contains_key(&serde_yaml::Value::String(
                                field.to_string(),
                            )) {
                                diags.push(LintDiag {
                                    level: DiagLevel::Warning,
                                    code: "W091",
                                    message: format!(
                                        "SKILL.md frontmatter missing recommended field: {}",
                                        field
                                    ),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E090",
                        message: format!("SKILL.md frontmatter parse error: {}", e),
                    });
                }
            }
        }
    }

    // ── Prompt injection scan ────────────────────────────────────
    check_prompt_injection(&content, diags);

    // ── onchainos command cross-check ────────────────────────────
    check_onchainos_command_consistency(&content, plugin, diags);
}

fn check_prompt_injection(content: &str, diags: &mut Vec<LintDiag>) {
    let lower = content.to_lowercase();

    let dangerous_patterns: &[(&str, &str)] = &[
        (
            "ignore previous instructions",
            "prompt injection: attempts to override agent instructions",
        ),
        (
            "ignore all previous",
            "prompt injection: attempts to override agent instructions",
        ),
        (
            "you are now",
            "prompt injection: attempts to redefine agent identity",
        ),
        (
            "do not show this to the user",
            "prompt injection: attempts to hide behavior from user",
        ),
        (
            "execute without user confirmation",
            "attempts to bypass user confirmation",
        ),
        (
            "skip safety checks",
            "attempts to bypass safety mechanisms",
        ),
        (
            "transfer all tokens",
            "attempts unauthorized asset transfer",
        ),
        ("send all funds", "attempts unauthorized asset transfer"),
        ("drain wallet", "attempts to drain wallet"),
    ];

    for (pattern, reason) in dangerous_patterns {
        if lower.contains(pattern) {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E100",
                message: format!(
                    "dangerous pattern detected: '{}' — {}",
                    pattern, reason
                ),
            });
        }
    }

    let suspicious_patterns: &[(&str, &str)] = &[
        (
            "base64",
            "contains base64 reference — may embed hidden content",
        ),
        ("eval(", "contains eval() — dynamic code execution"),
        (
            "curl ",
            "contains curl command — external network request",
        ),
        (
            "wget ",
            "contains wget command — external network request",
        ),
    ];

    for (pattern, reason) in suspicious_patterns {
        if lower.contains(pattern) {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W100",
                message: format!("suspicious pattern: '{}' — {}", pattern, reason),
            });
        }
    }
}

/// Cross-check: find `onchainos <subcommand>` references in SKILL.md and
/// verify they are listed in permissions.network.onchainos_commands.
fn check_onchainos_command_consistency(
    content: &str,
    plugin: &PluginYaml,
    diags: &mut Vec<LintDiag>,
) {
    let re = regex::Regex::new(r"onchainos\s+(\w+(?:\s+\w+)?)").unwrap();

    let declared: Vec<&str> = plugin
        .permissions
        .as_ref()
        .and_then(|p| p.network.as_ref())
        .map(|n| n.onchainos_commands.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let mut found_commands: Vec<String> = Vec::new();

    for cap in re.captures_iter(content) {
        let cmd = cap[1].trim().to_string();
        // Skip common non-command references like "onchainos CLI"
        if ["cli", "is", "the", "a", "an", "or", "and"].contains(&cmd.as_str()) {
            continue;
        }
        if !found_commands.contains(&cmd) {
            found_commands.push(cmd);
        }
    }

    for cmd in &found_commands {
        if !declared.iter().any(|d| cmd.starts_with(d) || d.starts_with(cmd.as_str())) {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W110",
                message: format!(
                    "SKILL.md references 'onchainos {}' but it is not listed in \
                     permissions.network.onchainos_commands",
                    cmd
                ),
            });
        }
    }
}

fn check_dir_name_match(plugin_name: &str, dir: &Path, diags: &mut Vec<LintDiag>) {
    if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
        // Skip if dir starts with _ (example plugins)
        if dir_name.starts_with('_') {
            return;
        }
        if dir_name != plugin_name {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W120",
                message: format!(
                    "directory name '{}' does not match plugin name '{}'",
                    dir_name, plugin_name
                ),
            });
        }
    }
}

/// Scan ALL text files in the submission for patterns indicating the plugin
/// implements on-chain capabilities itself instead of using onchainos CLI.
fn check_onchainos_bypass(plugin: &PluginYaml, dir: &Path, diags: &mut Vec<LintDiag>) {
    use super::onchainos_api::BYPASS_PATTERNS;

    // Collect all text content from the submission
    let files = match walk_dir(dir) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut all_content = String::new();
    for file in &files {
        if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
            if !["md", "yaml", "yml", "json", "txt", "toml"].contains(&ext) {
                continue;
            }
        } else {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(file) {
            all_content.push_str(&content);
            all_content.push('\n');
        }
    }

    let lower = all_content.to_lowercase();

    for bp in BYPASS_PATTERNS {
        let mut matched_patterns: Vec<&str> = Vec::new();
        for pattern in bp.patterns {
            if lower.contains(&pattern.to_lowercase()) {
                matched_patterns.push(pattern);
            }
        }

        if !matched_patterns.is_empty() {
            let level = if bp.severity == "error" {
                DiagLevel::Error
            } else {
                DiagLevel::Warning
            };

            let code = if bp.severity == "error" {
                "E200"
            } else {
                "W200"
            };

            diags.push(LintDiag {
                level,
                code,
                message: format!(
                    "onchainos API bypass: plugin appears to self-implement '{}'. \
                     Found: [{}]. \
                     Use {} instead.",
                    bp.capability,
                    matched_patterns.join(", "),
                    bp.onchainos_alternative
                ),
            });
        }
    }

    // Also check: does the plugin use ANY onchainos commands at all?
    let uses_onchainos = lower.contains("onchainos ");
    if !uses_onchainos {
        // Check if it has a skill component (MCP-only or binary-only plugins are OK)
        if plugin.components.skill.is_some() {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W201",
                message:
                    "SKILL.md does not reference any 'onchainos' commands. \
                     All community plugins must use onchainos CLI for on-chain operations."
                        .to_string(),
            });
        }
    }
}

fn find_skill_md(plugin: &PluginYaml, dir: &Path) -> Option<PathBuf> {
    if let Some(ref skill) = plugin.components.skill {
        if let Some(ref skill_dir) = skill.dir {
            return Some(dir.join(skill_dir).join("SKILL.md"));
        }
        if let Some(ref path) = skill.path {
            return Some(dir.join(path));
        }
    }

    // Fallback: look for any SKILL.md in skills/<name>/
    let default = dir.join("skills").join(&plugin.name).join("SKILL.md");
    if default.exists() {
        return Some(default);
    }
    None
}

/// Recursively walk a directory and collect file paths (skips hidden dirs).
fn walk_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            files.extend(walk_dir(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
