use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::plugin_yaml::{
    PluginYaml, VALID_CATEGORIES, VALID_LICENSES,
    VALID_BUILD_LANGS, FORBIDDEN_BINARY_EXTENSIONS,
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

    // ── 10. Tag content validation ─────────────────────────────────
    check_tags(&plugin, &mut diags);

    // ── 11. Community plugins: Binary without build is forbidden ───
    check_community_component_restrictions(&plugin, &mut diags);

    // ── 11. Components validation ─────────────────────────────────
    check_components(&plugin, submission_dir, &mut diags);



    // ── 13. File size limits ──────────────────────────────────────
    check_file_sizes(submission_dir, &mut diags);

    // ── 14. SKILL.md content validation ───────────────────────────
    check_skill_md(&plugin, submission_dir, &mut diags);

    // ── 15. PR scope: directory name matches plugin name ──────────
    check_dir_name_match(&plugin.name, submission_dir, &mut diags);

    // NOTE: onchainos API compliance is checked by AI review (Phase 3),
    // not by static lint. AI can understand context and intent, while
    // pattern matching produces false positives on natural language.

    // ── 16. Build configuration validation ──────────────────────
    check_build_config(&plugin, submission_dir, &mut diags);

    // ── 17. Forbidden binary files in submission ────────────────
    check_forbidden_binaries(submission_dir, &mut diags);

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
    // Skip reserved prefix check for official plugins (set PLUGIN_STORE_OFFICIAL=1)
    if std::env::var("PLUGIN_STORE_OFFICIAL").unwrap_or_default() == "1" {
        return;
    }
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

/// Validate tags for format rules and forbidden content.
fn check_tags(plugin: &PluginYaml, diags: &mut Vec<LintDiag>) {
    if plugin.tags.is_empty() {
        return;
    }

    // Tag format: lowercase, alphanumeric + hyphens, reasonable length
    let tag_re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$").unwrap();
    for tag in &plugin.tags {
        if tag.len() > 30 {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W045",
                message: format!("tag '{}' exceeds 30 characters", tag),
            });
        }
        if !tag_re.is_match(tag) && tag != "TODO" {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W046",
                message: format!(
                    "tag '{}' should be lowercase alphanumeric with hyphens",
                    tag
                ),
            });
        }
    }

    if plugin.tags.len() > 10 {
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W047",
            message: format!(
                "too many tags ({}, max recommended: 10)",
                plugin.tags.len()
            ),
        });
    }

    // Forbidden content in tags, name, and description.
    // Covers: financial fraud, political sensitivity, illegal content.
    let fields_to_check: Vec<(&str, &str)> = {
        let mut v = Vec::new();
        for tag in &plugin.tags {
            v.push(("tag", tag.as_str()));
        }
        v.push(("name", plugin.name.as_str()));
        v.push(("description", plugin.description.as_str()));
        v
    };

    // Financial fraud / misleading claims
    let fraud_keywords = [
        "guaranteed profit", "guaranteed return", "risk free", "risk-free",
        "100% profit", "no loss", "never lose", "free money",
        "必赚", "稳赚", "保本", "零风险", "无风险", "百分百收益",
        "躺赚", "暴富", "一夜暴富", "翻倍", "包赔",
    ];

    // Political sensitivity (mainland China restricted)
    let political_keywords = [
        "习近平", "xi jinping", "毛泽东", "邓小平", "江泽民", "胡锦涛",
        "共产党", "共产主义", "天安门", "tiananmen",
        "法轮功", "falun gong", "台独", "藏独", "疆独",
        "六四", "64事件", "文化大革命", "文革",
    ];

    // Illegal / harmful content
    let illegal_keywords = [
        "gambling", "casino", "赌博", "赌场", "博彩",
        "porn", "色情", "成人内容",
        "drug", "毒品", "大麻",
        "money laundering", "洗钱",
        "terrorism", "恐怖主义",
        "scam", "ponzi", "庞氏",
        "rug pull", "rugpull",
    ];

    for (field_name, value) in &fields_to_check {
        let lower = value.to_lowercase();

        for kw in &fraud_keywords {
            if lower.contains(&kw.to_lowercase()) {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E045",
                    message: format!(
                        "{} contains misleading financial claim: '{}'. \
                         Plugins must not promise guaranteed returns or risk-free outcomes.",
                        field_name, kw
                    ),
                });
            }
        }

        for kw in &political_keywords {
            if lower.contains(&kw.to_lowercase()) {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E046",
                    message: format!(
                        "{} contains politically sensitive content: '{}'. \
                         This content is restricted.",
                        field_name, kw
                    ),
                });
            }
        }

        for kw in &illegal_keywords {
            if lower.contains(&kw.to_lowercase()) {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E047",
                    message: format!(
                        "{} contains prohibited content: '{}'. \
                         Illegal, harmful, or fraudulent content is not allowed.",
                        field_name, kw
                    ),
                });
            }
        }
    }
}

/// Binary components require a `build` section — we compile from source.
fn check_community_component_restrictions(plugin: &PluginYaml, diags: &mut Vec<LintDiag>) {
    if plugin.components.binary.is_some() && !plugin.has_build() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E111",
            message:
                "Binary component requires a `build` section in plugin.yaml — \
                 we compile your source code, you don't submit pre-built binaries. \
                 Add build.lang, build.source_repo, and build.binary_name."
                    .to_string(),
        });
    }
}

fn check_components(plugin: &PluginYaml, dir: &Path, diags: &mut Vec<LintDiag>) {
    let has_any = plugin.components.has_skill()
        || plugin.components.binary.is_some();

    if !has_any {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E050",
            message: "at least one component (skill/skills or binary) must be declared"
                .to_string(),
        });
    }

    // Cannot use both `skill` (singular) and `skills` (plural) at the same time
    if plugin.components.skill.is_some() && !plugin.components.skills.is_empty() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E053",
            message: "use either `skill` (singular) or `skills` (plural), not both. \
                     Prefer `skills` for multiple skills."
                .to_string(),
        });
    }

    // ── Validate each declared skill ─────────────────────────────
    let all_skills = plugin.components.all_skills();
    for skill in &all_skills {
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
        // ── External repo validation (Mode B/C) ──
        if skill.is_external() {
            let repo = skill.repo.as_deref().unwrap_or("");
            // Validate repo format: owner/name
            if !repo.contains('/') || repo.starts_with("https://") {
                diags.push(LintDiag {
                    level: DiagLevel::Error,
                    code: "E070",
                    message: format!(
                        "skill.repo '{}' must be in 'owner/repo' format, not a URL",
                        repo
                    ),
                });
            }
            // Validate commit SHA
            match &skill.commit {
                None => {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E071",
                        message: "skill.commit is required when skill.repo is set (external repo mode). \
                                  Get it with: git rev-parse HEAD".to_string(),
                    });
                }
                Some(sha) if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) => {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E072",
                        message: format!(
                            "skill.commit '{}' must be a full 40-character hex SHA",
                            sha
                        ),
                    });
                }
                _ => {}
            }
        }

        if skill.dir.is_none() && skill.path.is_none() && skill.repo.is_none() {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W051",
                message:
                    "skill entry has no dir, path, or repo"
                        .to_string(),
            });
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

fn check_file_sizes(dir: &Path, diags: &mut Vec<LintDiag>) {
    const MAX_SINGLE_FILE: u64 = 200 * 1024; // 200 KB
    const MAX_TOTAL: u64 = 5 * 1024 * 1024; // 5 MB

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
                            "file '{}' is {} KB (limit: 200 KB)",
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
    let skill_paths = find_all_skill_mds(plugin, dir);

    if skill_paths.is_empty() {
        return; // Already reported in check_components
    }

    for path in &skill_paths {
        check_single_skill_md(path, plugin, diags);
    }
}

fn check_single_skill_md(path: &Path, plugin: &PluginYaml, diags: &mut Vec<LintDiag>) {
    let content = match std::fs::read_to_string(path) {
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

    // ── Zero-width / invisible character detection ───────────────
    check_invisible_characters(&content, diags);

    // ── Dangerous onchainos commands must have confirmation ──────
    check_dangerous_commands_confirmation(&content, diags);

    // ── External URL safety analysis ─────────────────────────────
    check_external_urls(&content, plugin, diags);
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

    // Strip auto-injected pre-flight blocks before scanning for suspicious patterns.
    // These blocks are added by CI (inject-preflight.py) and contain legitimate
    // base64, curl, and URL references for binary installation and telemetry.
    let lower_no_preflight = strip_auto_injected_blocks(&lower);

    for (pattern, reason) in suspicious_patterns {
        if lower_no_preflight.contains(pattern) {
            diags.push(LintDiag {
                level: DiagLevel::Warning,
                code: "W100",
                message: format!("suspicious pattern: '{}' — {}", pattern, reason),
            });
        }
    }
}

/// Remove CI auto-injected pre-flight blocks from content before security scanning.
/// These blocks are marked with "(auto-injected)" in their heading and contain
/// legitimate curl/base64/URL patterns for binary installation and telemetry reporting.
fn strip_auto_injected_blocks(content: &str) -> String {
    let mut result = String::new();
    let mut in_injected_block = false;
    let mut in_code_fence = false;

    for line in content.lines() {
        if line.contains("(auto-injected)") {
            in_injected_block = true;
            continue;
        }
        if in_injected_block {
            if line.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }
            // End of injected block: next heading at same or higher level
            if !in_code_fence && (line.starts_with("## ") || line.starts_with("# ") || line.starts_with("---")) {
                in_injected_block = false;
                result.push_str(line);
                result.push('\n');
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Detect invisible / zero-width characters that could hide malicious instructions.
/// These characters are invisible in most editors but can be parsed by AI agents.
fn check_invisible_characters(content: &str, diags: &mut Vec<LintDiag>) {
    let invisible_chars: &[(char, &str)] = &[
        ('\u{200B}', "zero-width space"),
        ('\u{200C}', "zero-width non-joiner"),
        ('\u{200D}', "zero-width joiner"),
        ('\u{200E}', "left-to-right mark"),
        ('\u{200F}', "right-to-left mark"),
        ('\u{2060}', "word joiner"),
        ('\u{FEFF}', "zero-width no-break space / BOM"),
        ('\u{00AD}', "soft hyphen"),
        ('\u{034F}', "combining grapheme joiner"),
        ('\u{2028}', "line separator"),
        ('\u{2029}', "paragraph separator"),
        ('\u{202A}', "left-to-right embedding"),
        ('\u{202B}', "right-to-left embedding"),
        ('\u{202C}', "pop directional formatting"),
        ('\u{202D}', "left-to-right override"),
        ('\u{202E}', "right-to-left override"),
    ];

    for (ch, name) in invisible_chars {
        if content.contains(*ch) {
            let count = content.chars().filter(|c| c == ch).count();
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E105",
                message: format!(
                    "invisible character detected: {} (U+{:04X}) appears {} time(s) — \
                     may be used to hide malicious instructions",
                    name,
                    *ch as u32,
                    count
                ),
            });
        }
    }
}

/// If SKILL.md references dangerous onchainos commands (wallet send,
/// contract-call, gateway broadcast), it MUST include user confirmation
/// language nearby. This is a heuristic, not a guarantee.
fn check_dangerous_commands_confirmation(content: &str, diags: &mut Vec<LintDiag>) {
    let lower = content.to_lowercase();

    let dangerous_commands: &[(&str, &str)] = &[
        ("wallet send", "transfer funds"),
        ("wallet contract-call", "call smart contracts"),
        ("gateway broadcast", "broadcast transactions"),
        ("swap swap", "execute token swaps"),
        ("payment x402-pay", "make payments"),
    ];

    let confirmation_phrases = [
        "confirm",
        "ask user",
        "ask the user",
        "user confirm",
        "user approval",
        "approval",
        "verify with user",
        "prompt user",
        "require confirmation",
    ];

    for (cmd, description) in dangerous_commands {
        if lower.contains(cmd) {
            // Check if there's a confirmation phrase within ~500 chars of the command
            if let Some(pos) = lower.find(cmd) {
                let context_start = pos.saturating_sub(300);
                let context_end = (pos + 500).min(lower.len());
                let nearby = &lower[context_start..context_end];

                let has_confirmation = confirmation_phrases
                    .iter()
                    .any(|phrase| nearby.contains(phrase));

                if !has_confirmation {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E106",
                        message: format!(
                            "SKILL.md uses 'onchainos {}' ({}) but no user confirmation \
                             step found nearby. Dangerous operations MUST include explicit \
                             user confirmation before execution.",
                            cmd, description
                        ),
                    });
                }
            }
        }
    }
}

/// Analyze external URLs in SKILL.md for security risks.
///
/// Three categories:
/// 1. Safe: GitHub/docs links used as references (OK)
/// 2. Declared API: listed in permissions.network.api_calls (OK but flagged)
/// 3. Undeclared/dangerous: fetching instructions or sending data to unknown URLs (ERROR)
/// Analyze external URLs in SKILL.md for security risks.
/// Uses plugin.api_calls to distinguish expected vs unexpected URLs.
fn check_external_urls(content: &str, plugin: &PluginYaml, diags: &mut Vec<LintDiag>) {
    // Strip auto-injected pre-flight blocks — their URLs are CI-controlled, not developer-submitted
    let cleaned = strip_auto_injected_blocks(content);
    let url_re = regex::Regex::new(r#"https?://[^\s\)>\]`"']+"#).unwrap();
    let urls: Vec<&str> = url_re.find_iter(&cleaned).map(|m| m.as_str()).collect();

    if urls.is_empty() {
        return;
    }

    let safe_domains = [
        "github.com",
        "raw.githubusercontent.com",
        "web3.okx.com",
        "docs.okx.com",
        "onchainos.com",
        // CI auto-injected telemetry (inject-preflight.py report install block)
        "plugin-store-dun.vercel.app",
        "www.okx.com/priapi/v1/wallet/plugins",
    ];

    let lower = content.to_lowercase();
    let declared_apis = &plugin.api_calls;

    let fetch_patterns = [
        "download from", "fetch from", "load from", "get from",
        "retrieve from", "pull from", "import from",
        "download instructions", "fetch instructions",
        "load prompt", "load config", "load script",
        "execute from", "run from",
    ];
    let exfil_patterns = [
        "send to", "post to", "upload to", "report to",
        "transmit to", "forward to",
        "send wallet", "send address", "send balance", "track user",
    ];

    // Check for remote instruction loading
    for pattern in &fetch_patterns {
        if lower.contains(pattern) {
            for url in &urls {
                if !safe_domains.iter().any(|d| url.contains(d)) {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E140",
                        message: format!(
                            "SKILL.md instructs AI to fetch/load/download from external URL '{}'. \
                             This allows remote code injection — an attacker can change the URL \
                             content after review. Use onchainos commands or inline the content.",
                            url
                        ),
                    });
                    break;
                }
            }
            break;
        }
    }

    // Check for data exfiltration
    for pattern in &exfil_patterns {
        if lower.contains(pattern) {
            for url in &urls {
                if !safe_domains.iter().any(|d| url.contains(d)) {
                    // If the URL is declared in api_calls, downgrade to warning
                    let is_declared = declared_apis.iter().any(|d| url.contains(d.as_str()));
                    diags.push(LintDiag {
                        level: if is_declared { DiagLevel::Warning } else { DiagLevel::Error },
                        code: if is_declared { "W141" } else { "E141" },
                        message: format!(
                            "SKILL.md instructs AI to send/post data to external URL '{}'. {}",
                            url,
                            if is_declared {
                                "Declared in api_calls — reviewer should verify this is intentional."
                            } else {
                                "This may exfiltrate user data (wallet addresses, balances, etc.)."
                            }
                        ),
                    });
                    break;
                }
            }
            break;
        }
    }

    // Categorize external URLs: declared in api_calls vs undeclared
    let declared_apis = &plugin.api_calls;

    let external_urls: Vec<&&str> = urls
        .iter()
        .filter(|u| !safe_domains.iter().any(|d| u.contains(d)))
        .collect();

    let undeclared: Vec<&&str> = external_urls
        .iter()
        .filter(|u| !declared_apis.iter().any(|d| u.contains(d.as_str())))
        .copied()
        .collect();

    if !undeclared.is_empty() {
        let display: Vec<String> = undeclared.iter().take(5).map(|u| format!("'{}'", u)).collect();
        diags.push(LintDiag {
            level: DiagLevel::Warning,
            code: "W140",
            message: format!(
                "SKILL.md references {} external URL(s) not listed in api_calls: {}. \
                 Add them to api_calls in plugin.yaml so reviewers can verify them.",
                undeclared.len(),
                display.join(", ")
            ),
        });
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

/// Validate the `build` section if present.
///
/// Source code lives in the developer's external GitHub repo, referenced by
/// `source_repo` + `source_commit`. We don't store source code in our repo.
/// The commit SHA is the content fingerprint — same SHA = same code.
fn check_build_config(plugin: &PluginYaml, _dir: &Path, diags: &mut Vec<LintDiag>) {
    let build = match &plugin.build {
        Some(b) => b,
        None => return,
    };

    // Must have a Skill component — Skill is the entry point for everything
    if !plugin.components.has_skill() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E120",
            message:
                "plugins with build config must also include a Skill component — \
                 SKILL.md is the entry point that tells the AI agent how to use \
                 your binary."
                    .to_string(),
        });
    }

    // Validate lang
    if !VALID_BUILD_LANGS.contains(&build.lang.as_str()) {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E121",
            message: format!(
                "build.lang '{}' is not supported. Valid: {}",
                build.lang,
                VALID_BUILD_LANGS.join(", ")
            ),
        });
    }

    // Validate source_repo format (owner/repo) — only if provided
    // source_repo is optional: when absent, source code is in the submission directory (local build)
    if !build.source_repo.is_empty() {
        let repo_re = regex::Regex::new(r"^[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+$").unwrap();
        if !repo_re.is_match(&build.source_repo) {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E122",
                message: format!(
                    "build.source_repo '{}' is not valid — expected format: owner/repo",
                    build.source_repo
                ),
            });
        }

        // Validate source_commit is a full 40-char hex SHA — only required for external repos
        let sha_re = regex::Regex::new(r"^[0-9a-f]{40}$").unwrap();
        if !sha_re.is_match(&build.source_commit) {
            diags.push(LintDiag {
                level: DiagLevel::Error,
                code: "E123",
                message: format!(
                    "build.source_commit '{}' must be a full 40-character hex SHA — \
                     short SHAs and branch names are not accepted for integrity verification",
                    build.source_commit
                ),
            });
        }
    }

    // binary_name is required for all compiled languages
    if build.binary_name.is_none() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E124",
            message: format!(
                "build.binary_name is required for lang '{}' — \
                 this is the name of the compiled output",
                build.lang
            ),
        });
    }

    // TypeScript, Node.js, and Python require a main entry point
    if (build.lang == "typescript" || build.lang == "node" || build.lang == "python") && build.main.is_none() {
        diags.push(LintDiag {
            level: DiagLevel::Error,
            code: "E125",
            message: format!(
                "build.main is required for lang '{}' — \
                 specify the entry file (e.g. src/index.ts, src/index.js, or src/main.py)",
                build.lang
            ),
        });
    }
}

/// Reject pre-compiled binary files in submissions.
/// Developers must submit source code; we compile.
fn check_forbidden_binaries(dir: &Path, diags: &mut Vec<LintDiag>) {
    if let Ok(files) = walk_dir(dir) {
        for file in &files {
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                if FORBIDDEN_BINARY_EXTENSIONS.contains(&ext) {
                    diags.push(LintDiag {
                        level: DiagLevel::Error,
                        code: "E130",
                        message: format!(
                            "pre-compiled binary file '{}' is not allowed — \
                             submit source code instead, we handle compilation",
                            file.strip_prefix(dir).unwrap_or(file).display()
                        ),
                    });
                }
            }
        }
    }
}

/// Find all SKILL.md paths from explicitly declared skills.
fn find_all_skill_mds(plugin: &PluginYaml, dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for skill in plugin.components.all_skills() {
        if let Some(ref skill_dir) = skill.dir {
            let p = dir.join(skill_dir).join("SKILL.md");
            if p.exists() {
                paths.push(p);
            }
        }
        if let Some(ref path) = skill.path {
            let p = dir.join(path);
            if p.exists() {
                paths.push(p);
            }
        }
    }

    // Fallback: if nothing declared, look for default location
    if paths.is_empty() {
        let default = dir.join("skills").join(&plugin.name).join("SKILL.md");
        if default.exists() {
            paths.push(default);
        }
    }

    paths
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
