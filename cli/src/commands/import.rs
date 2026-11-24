use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

/// Import a Claude marketplace-compatible repo as a plugin-store submission.
/// Reads .claude-plugin/plugin.json + skills/, generates plugin.yaml,
/// forks community repo, creates branch, commits, and opens a PR.
pub async fn execute(repo: &str, yes: bool) -> Result<()> {
    let repo = repo.trim_start_matches("https://github.com/").trim_end_matches(".git");
    if !repo.contains('/') || repo.split('/').count() != 2 {
        bail!("Invalid repo format. Use: plugin-store import owner/repo");
    }

    println!("{}", "Plugin Store Import".bold());
    println!("Importing from: {}", repo.cyan());
    println!();

    // ── Pre-flight dependency checks ────────────────────────────
    preflight_checks()?;

    // ── Step 1: Read repo metadata via GitHub API ───────────────
    println!("Reading repository...");

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Get HEAD commit SHA
    let commit_sha = get_head_sha(&client, repo).await
        .context("Failed to get HEAD commit. Is the repo public?")?;
    println!("  Commit: {}", &commit_sha[..12]);

    // Try to read .claude-plugin/plugin.json
    let plugin_json = fetch_file(&client, repo, &commit_sha, ".claude-plugin/plugin.json").await;
    let (name, description, version, author) = if let Ok(content) = &plugin_json {
        parse_plugin_json(content)?
    } else {
        // Fallback: derive from repo name
        let repo_name = repo.split('/').last().unwrap_or("unknown");
        (
            repo_name.to_lowercase().replace('_', "-"),
            format!("Plugin imported from {}", repo),
            "1.0.0".to_string(),
            repo.split('/').next().unwrap_or("unknown").to_string(),
        )
    };
    println!("  Name: {}", name.green());
    println!("  Version: {}", version);

    // Detect SKILL.md location
    let skill_path = detect_skill_path(&client, repo, &commit_sha).await?;
    println!("  Skill: {}", skill_path);

    // Detect build language
    let build_lang = detect_build_lang(&client, repo, &commit_sha).await;
    if let Some(ref lang) = build_lang {
        println!("  Build: {} detected", lang.yellow());
        check_build_deps(lang)?;
    }

    // ── Step 2: Confirm with user ───────────────────────────────
    if !yes {
        println!();
        println!("This will create a PR to submit '{}' to plugin-store.", name.bold());
        print!("Continue? [Y/n] ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("n") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // ── Step 3: Generate plugin.yaml ────────────────────────────
    let github_user = get_gh_username()?;
    let mut yaml_content = format!(
        r#"schema_version: 1
name: {name}
version: "{version}"
description: "{description}"
author:
  name: "{author}"
  github: "{github_user}"
license: MIT
category: utility
tags: []

components:
  skill:
    repo: "{repo}"
    commit: "{commit_sha}"

api_calls: []
"#
    );

    // Add build section if compiled language detected
    if let Some(ref lang) = build_lang {
        let binary_name = &name;
        let build_section = match lang.as_str() {
            "rust" => format!(
                "\nbuild:\n  lang: rust\n  source_repo: \"{repo}\"\n  source_commit: \"{commit_sha}\"\n  binary_name: \"{binary_name}\"\n"
            ),
            "go" => format!(
                "\nbuild:\n  lang: go\n  source_repo: \"{repo}\"\n  source_commit: \"{commit_sha}\"\n  binary_name: \"{binary_name}\"\n"
            ),
            "typescript" => format!(
                "\nbuild:\n  lang: typescript\n  source_repo: \"{repo}\"\n  source_commit: \"{commit_sha}\"\n  binary_name: \"{binary_name}\"\n  main: src/index.js\n"
            ),
            "node" => format!(
                "\nbuild:\n  lang: node\n  source_repo: \"{repo}\"\n  source_commit: \"{commit_sha}\"\n  binary_name: \"{binary_name}\"\n  main: src/index.js\n"
            ),
            "python" => format!(
                "\nbuild:\n  lang: python\n  source_repo: \"{repo}\"\n  source_commit: \"{commit_sha}\"\n  binary_name: \"{binary_name}\"\n  main: src/main.py\n"
            ),
            _ => String::new(),
        };
        yaml_content.push_str(&build_section);
    }

    // ── Step 4: Fork + branch + commit + PR via gh CLI ──────────
    println!();
    println!("Creating submission...");

    let community_repo = "okx/plugin-store";

    // Fork (idempotent — gh handles already-forked case)
    run_cmd("gh", &["repo", "fork", community_repo, "--clone=false"])?;

    // Clone fork
    let fork_repo = format!("{}/{}", github_user, "plugin-store");
    let work_dir = format!("/tmp/import-{}", name);
    let _ = std::fs::remove_dir_all(&work_dir);
    run_cmd("gh", &["repo", "clone", &fork_repo, &work_dir, "--", "--depth=1"])?;

    // Sync fork with upstream
    let git = |args: &[&str]| -> Result<()> {
        let status = Command::new("git")
            .args(args)
            .current_dir(&work_dir)
            .status()?;
        if !status.success() {
            bail!("git {} failed", args.join(" "));
        }
        Ok(())
    };

    // gh clone of a fork may auto-add 'upstream'; ignore error if already exists
    let _ = git(&["remote", "add", "upstream", &format!("https://github.com/{}.git", community_repo)]);
    let _ = git(&["remote", "set-url", "upstream", &format!("https://github.com/{}.git", community_repo)]);
    git(&["fetch", "upstream", "main"])?;
    git(&["reset", "--hard", "upstream/main"])?;
    git(&["push", "origin", "main", "--force"])?;

    // Create branch
    let branch = format!("submit/{}", name);
    git(&["checkout", "-b", &branch])?;

    // Write files
    let sub_dir = format!("{}/skills/{}", work_dir, name);
    std::fs::create_dir_all(&sub_dir)?;
    std::fs::write(format!("{}/plugin.yaml", sub_dir), &yaml_content)?;
    std::fs::write(
        format!("{}/LICENSE", sub_dir),
        "MIT License\n\nCopyright (c) 2026\n\nPermission is hereby granted, free of charge, to any person...\n",
    )?;
    std::fs::write(
        format!("{}/README.md", sub_dir),
        format!("# {}\n\n{}\n\n## Install\n\n```bash\nnpx skills add okx/plugin-store --name {}\n```\n", name, description, name),
    )?;

    // Commit and push
    git(&["add", &format!("skills/{}", name)])?;
    git(&["commit", "-m", &format!("[new-plugin] {} v{}", name, version)])?;
    git(&["push", "origin", &branch, "--force"])?;

    // Create PR
    let pr_output = Command::new("gh")
        .args([
            "pr", "create",
            "--repo", community_repo,
            "--head", &format!("{}:{}", github_user, branch),
            "--title", &format!("[new-plugin] {} v{} — imported from Claude marketplace", name, version),
            "--body", &format!("Imported from `{}` via `plugin-store import`.\n\nSource: https://github.com/{}", repo, repo),
        ])
        .output()?;

    let pr_url = String::from_utf8_lossy(&pr_output.stdout).trim().to_string();
    if pr_output.status.success() && !pr_url.is_empty() {
        println!();
        println!("{} PR created: {}", "Done!".green().bold(), pr_url);
        println!("Wait for CI checks and reviewer approval.");
    } else {
        let err = String::from_utf8_lossy(&pr_output.stderr);
        bail!("Failed to create PR: {}", err);
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&work_dir);

    Ok(())
}

// ── Helper functions ────────────────────────────────────────────

fn preflight_checks() -> Result<()> {
    println!("Pre-flight checks:");

    // gh CLI
    let gh_ver = cmd_output("gh", &["--version"]);
    match gh_ver {
        Ok(v) => println!("  {} gh — {}", "✓".green(), v.lines().next().unwrap_or("?")),
        Err(_) => bail!("gh CLI not found.\n  Install: brew install gh  or  https://cli.github.com"),
    }

    // gh auth
    let gh_auth = cmd_output("gh", &["auth", "status"]);
    match gh_auth {
        Ok(_) => println!("  {} gh auth — logged in", "✓".green()),
        Err(_) => bail!("Not logged in to GitHub.\n  Run: gh auth login"),
    }

    // git
    let git_ver = cmd_output("git", &["--version"]);
    match git_ver {
        Ok(v) => println!("  {} git — {}", "✓".green(), v.trim()),
        Err(_) => bail!("git not found.\n  Install: brew install git  or  https://git-scm.com"),
    }

    println!();
    Ok(())
}

fn check_build_deps(lang: &str) -> Result<()> {
    match lang {
        "rust" => {
            if cmd_output("cargo", &["--version"]).is_err() {
                println!("  {} cargo — not found (needed for Rust build)", "✗".red());
                println!("    Install: https://rustup.rs");
            }
        }
        "go" => {
            if cmd_output("go", &["version"]).is_err() {
                println!("  {} go — not found (needed for Go build)", "✗".red());
                println!("    Install: https://go.dev/dl");
            }
        }
        "typescript" | "node" => {
            if cmd_output("npm", &["--version"]).is_err() {
                println!("  {} npm — not found (needed for TS/Node)", "✗".red());
                println!("    Install: brew install node  or  https://nodejs.org");
            }
        }
        "python" => {
            if cmd_output("pip3", &["--version"]).is_err() && cmd_output("pip", &["--version"]).is_err() {
                println!("  {} pip — not found (needed for Python)", "✗".red());
                println!("    Install: brew install python3  or  https://python.org");
            }
        }
        _ => {}
    }
    Ok(())
}

async fn get_head_sha(client: &reqwest::Client, repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/commits/HEAD", repo);
    let resp: serde_json::Value = client
        .get(&url)
        .header("User-Agent", "plugin-store")
        .send().await?
        .error_for_status()?
        .json().await?;
    resp["sha"].as_str()
        .map(|s| s.to_string())
        .context("No SHA in response")
}

async fn fetch_file(client: &reqwest::Client, repo: &str, sha: &str, path: &str) -> Result<String> {
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        repo, sha, path
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "plugin-store")
        .send().await?
        .error_for_status()?
        .text().await?;
    Ok(resp)
}

fn parse_plugin_json(content: &str) -> Result<(String, String, String, String)> {
    let v: serde_json::Value = serde_json::from_str(content)?;
    let name = v["name"].as_str().unwrap_or("unknown").to_string();
    let desc = v["description"].as_str().unwrap_or("").to_string();
    let version = v["version"].as_str().unwrap_or("1.0.0").to_string();
    let author = v["author"]["name"].as_str()
        .or_else(|| v["author"].as_str())
        .unwrap_or("unknown").to_string();
    Ok((name, desc, version, author))
}

async fn detect_skill_path(client: &reqwest::Client, repo: &str, sha: &str) -> Result<String> {
    // Try skills/<name>/SKILL.md first (Claude marketplace standard)
    let tree_url = format!(
        "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
        repo, sha
    );
    let resp: serde_json::Value = client
        .get(&tree_url)
        .header("User-Agent", "plugin-store")
        .send().await?
        .error_for_status()?
        .json().await?;

    if let Some(tree) = resp["tree"].as_array() {
        for item in tree {
            if let Some(path) = item["path"].as_str() {
                if path.ends_with("SKILL.md") || path.ends_with("skill.md") {
                    return Ok(path.to_string());
                }
            }
        }
    }

    bail!("No SKILL.md found in repository {}", repo);
}

async fn detect_build_lang(client: &reqwest::Client, repo: &str, sha: &str) -> Option<String> {
    // Check for language-specific files
    for (file, lang) in &[
        ("Cargo.toml", "rust"),
        ("go.mod", "go"),
        ("package.json", "node"),
        ("pyproject.toml", "python"),
    ] {
        if fetch_file(client, repo, sha, file).await.is_ok() {
            return Some(lang.to_string());
        }
    }
    None
}

fn get_gh_username() -> Result<String> {
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()?;
    if !output.status.success() {
        bail!("Failed to get GitHub username. Run: gh auth login");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn cmd_output(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;
    if !output.status.success() {
        bail!("command failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context(format!("Failed to run {}", cmd))?;
    if !status.success() {
        bail!("{} {} failed with exit code {:?}", cmd, args.join(" "), status.code());
    }
    Ok(())
}
