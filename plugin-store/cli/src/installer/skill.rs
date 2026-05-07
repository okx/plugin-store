use anyhow::{Context, Result};
use std::path::Path;

use crate::registry::models::{DiscoveredMcp, DiscoveredSkill};

pub struct SkillInstaller;

/// Result of scanning a repo tree for skills and MCP servers
pub struct DiscoverResult {
    pub skills: Vec<DiscoveredSkill>,
    pub mcps: Vec<DiscoveredMcp>,
}

impl SkillInstaller {
    /// Download a single file from GitHub raw URL.
    /// `git_ref` is a branch name, tag, or commit SHA (e.g. `"main"` or `"abc1234..."`).
    pub async fn download_from_github(repo: &str, path: &str, git_ref: &str) -> Result<String> {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, git_ref, path
        );
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let resp = client
            .get(&url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()
            .context(format!("Failed to download skill from {}", url))?;
        let content = resp.text().await?;
        Ok(content)
    }

    /// Write a single SKILL.md to the skill directory (legacy mode)
    pub fn write_skill(skill_dir: &Path, content: &str) -> Result<()> {
        std::fs::create_dir_all(skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, content)?;
        Ok(())
    }

    /// Fetch the full file tree of a GitHub repo at the given ref.
    /// Falls back to ZIP download if the API returns 403 (rate limit).
    async fn fetch_repo_tree(repo: &str, git_ref: &str) -> Result<Vec<String>> {
        let url = format!(
            "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
            repo, git_ref
        );
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let resp = client
            .get(&url)
            .header("User-Agent", "plugin-store")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Self::fetch_repo_tree_via_zip(repo, git_ref).await;
        }

        let resp = resp
            .error_for_status()
            .context(format!("Failed to fetch repo tree for {}", repo))?;

        let tree: serde_json::Value = resp.json().await?;
        let entries = tree["tree"]
            .as_array()
            .context("Invalid tree response")?;

        let paths: Vec<String> = entries
            .iter()
            .filter(|e| e["type"].as_str() == Some("blob"))
            .filter_map(|e| e["path"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(paths)
    }

    /// Fallback: download repo ZIP and extract file paths from it.
    async fn fetch_repo_tree_via_zip(repo: &str, git_ref: &str) -> Result<Vec<String>> {
        let url = format!("https://github.com/{}/archive/{}.zip", repo, git_ref);
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let bytes = client
            .get(&url)
            .header("User-Agent", "plugin-store")
            .send()
            .await?
            .error_for_status()
            .context(format!("Failed to download ZIP for {}", repo))?
            .bytes()
            .await?;

        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .context("Failed to open ZIP archive")?;

        // GitHub ZIP has a top-level prefix like "repo-name-ref/"
        // Strip that prefix to get paths relative to repo root.
        let mut paths = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name().to_string();
            // skip directories
            if name.ends_with('/') {
                continue;
            }
            // strip the first path component (e.g. "repo-main/")
            if let Some(stripped) = name.splitn(2, '/').nth(1) {
                if !stripped.is_empty() {
                    paths.push(stripped.to_string());
                }
            }
        }

        Ok(paths)
    }

    /// Install all files under a specific directory prefix in a repo.
    /// Downloads every file whose path starts with `dir/`, preserving structure.
    pub async fn install_from_dir(
        repo: &str,
        dir: &str,
        agent_skill_dir: &Path,
        git_ref: &str,
    ) -> Result<usize> {
        let resolved_dir = dir.to_string();

        let all_paths = Self::fetch_repo_tree(repo, git_ref).await?;
        let prefix = format!("{}/", resolved_dir.trim_end_matches('/'));

        let files: Vec<&String> = all_paths
            .iter()
            .filter(|p| p.starts_with(&prefix))
            .collect();

        if files.is_empty() {
            anyhow::bail!("No files found under '{}' in repo '{}'", dir, repo);
        }

        std::fs::create_dir_all(agent_skill_dir)?;

        for file_path in &files {
            let content = Self::download_from_github(repo, file_path, git_ref).await?;
            let relative = file_path.strip_prefix(&prefix).unwrap_or(file_path);
            let target = agent_skill_dir.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &content)?;
        }

        Ok(files.len())
    }

    /// Auto-discover all skills in a repo by scanning for SKILL.md files.
    pub async fn discover_skills(repo: &str, git_ref: &str) -> Result<Vec<DiscoveredSkill>> {
        let all_paths = Self::fetch_repo_tree(repo, git_ref).await?;
        Ok(Self::extract_skills(&all_paths, repo))
    }

    /// Auto-discover skills AND MCP servers from a repo.
    /// Scans for SKILL.md files and .mcp.json files (Vercel plugin convention).
    pub async fn discover_all(repo: &str, git_ref: &str) -> Result<DiscoverResult> {
        let all_paths = Self::fetch_repo_tree(repo, git_ref).await?;

        let skills = Self::extract_skills(&all_paths, repo);
        let mcps = Self::extract_mcps(repo, &all_paths, git_ref).await?;

        Ok(DiscoverResult { skills, mcps })
    }

    /// Extract skill entries from a list of file paths
    fn extract_skills(all_paths: &[String], repo: &str) -> Vec<DiscoveredSkill> {
        let skill_files: Vec<&String> = all_paths
            .iter()
            .filter(|p| p.ends_with("/SKILL.md") || *p == "SKILL.md")
            .collect();

        let mut skills = Vec::new();

        for skill_path in &skill_files {
            let skill_dir = if *skill_path == "SKILL.md" {
                ""
            } else {
                skill_path.strip_suffix("/SKILL.md").unwrap_or("")
            };

            let name = if skill_dir.is_empty() {
                repo.split('/').last().unwrap_or("skill").to_string()
            } else {
                skill_dir.split('/').last().unwrap_or("skill").to_string()
            };

            let prefix = if skill_dir.is_empty() {
                String::new()
            } else {
                format!("{}/", skill_dir)
            };

            let files: Vec<String> = all_paths
                .iter()
                .filter(|p| {
                    if prefix.is_empty() {
                        *p == "SKILL.md" || p.starts_with("references/")
                    } else {
                        p.starts_with(&prefix)
                    }
                })
                .cloned()
                .collect();

            skills.push(DiscoveredSkill { name, files });
        }

        skills
    }

    /// Find all .mcp.json files in the tree, download and parse them
    async fn extract_mcps(repo: &str, all_paths: &[String], git_ref: &str) -> Result<Vec<DiscoveredMcp>> {
        let mcp_files: Vec<&String> = all_paths
            .iter()
            .filter(|p| p.ends_with(".mcp.json"))
            .collect();

        let mut mcps = Vec::new();

        for mcp_path in mcp_files {
            let content = Self::download_from_github(repo, mcp_path, git_ref).await?;
            let parsed: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Parse mcpServers object: { "name": { "command": "...", "args": [...], "env": {...} } }
            if let Some(servers) = parsed.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in servers {
                    let command = config
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args: Vec<String> = config
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let env: Vec<String> = config
                        .get("env")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.keys().cloned().collect())
                        .unwrap_or_default();

                    if !command.is_empty() {
                        mcps.push(DiscoveredMcp {
                            name: name.clone(),
                            command,
                            args,
                            env,
                        });
                    }
                }
            }
        }

        Ok(mcps)
    }

    /// Download all files for a discovered skill and write them preserving directory structure.
    pub async fn install_discovered_skill(
        repo: &str,
        skill: &DiscoveredSkill,
        agent_skill_dir: &Path,
        git_ref: &str,
    ) -> Result<()> {
        std::fs::create_dir_all(agent_skill_dir)?;

        let skill_md = skill
            .files
            .iter()
            .find(|f| f.ends_with("SKILL.md"))
            .context("No SKILL.md in discovered skill")?;

        let base_prefix = if skill_md == "SKILL.md" {
            String::new()
        } else {
            let dir = skill_md.strip_suffix("/SKILL.md").unwrap_or("");
            format!("{}/", dir)
        };

        for file_path in &skill.files {
            let content = Self::download_from_github(repo, file_path, git_ref).await?;

            let relative = file_path
                .strip_prefix(&base_prefix)
                .unwrap_or(file_path);

            let target = agent_skill_dir.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &content)?;
        }

        Ok(())
    }
}
