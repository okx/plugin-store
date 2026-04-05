use anyhow::{bail, Result};
use colored::Colorize;
use plugin_store::agent::{detect_agents, get_adapter, AgentKind};
use plugin_store::installer::{skill::SkillInstaller, mcp::McpInstaller, binary::BinaryInstaller, python::PythonInstaller, npm::NpmInstaller};
use plugin_store::registry::RegistryManager;
use plugin_store::state::StateManager;
use plugin_store::state::models::{InstalledPlugin, InstalledAgent};
use plugin_store::utils::ui;
use plugin_store::stats;

pub async fn execute(
    name: &str,
    skill_only: bool,
    mcp_only: bool,
    agent_filter: Option<&str>,
    yes: bool,
) -> Result<()> {
    let registry_mgr = RegistryManager::new();
    let registry = registry_mgr.get_registry(false).await?;
    let stats_url = registry.stats_url.clone();
    let plugin = registry.plugins.into_iter().find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found. Run `plugin-store search <keyword>` to find plugins.", name))?;

    // Load state once; reuse for all lookups below
    let mut state_mgr = StateManager::new();
    let already_installed = state_mgr.find(name)?.is_some();

    // Community warning (skip for updates of already-installed plugins, or when --yes is passed)
    if plugin.source == "community" && !already_installed && !yes {
        use std::io::IsTerminal;
        if !std::io::stdout().is_terminal() {
            bail!(
                "'{}' is a community plugin. Re-run with --yes to confirm: \
                 plugin-store install {} --yes",
                plugin.name, plugin.name
            );
        }
        if !ui::confirm_community_plugin(&plugin.name) {
            println!("Installation cancelled.");
            return Ok(());
        }
    }

    // Determine target agents
    let target_agents: Vec<AgentKind> = if let Some(agent_id) = agent_filter {
        let kind = AgentKind::from_id(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown agent '{}'. Valid: claude-code, cursor, openclaw", agent_id))?;
        vec![kind]
    } else {
        let detected = detect_agents();
        if detected.is_empty() {
            bail!("No supported agents detected.");
        }
        use std::io::IsTerminal;
        if std::io::stdout().is_terminal() {
            let selected = ui::select_agents(&detected);
            if selected.is_empty() {
                bail!("No agents selected.");
            }
            selected.iter().map(|&i| detected[i].kind.clone()).collect()
        } else {
            // Non-interactive: install to all detected agents
            detected.iter().map(|a| a.kind.clone()).collect()
        }
    };

    println!("\nInstalling {} {}...", plugin.name.bold(), plugin.version);

    let existing = state_mgr.find(name)?;
    let mut installed_agents = Vec::new();
    let mut components = Vec::new();

    // Install binary once (shared across agents)
    let mut binary_path_shared: Option<String> = None;
    if !skill_only && !mcp_only {
        if let Some(ref bin) = plugin.components.binary {
            let install_dir = bin.install_dir.as_deref().unwrap_or("~/.plugin-store/bin/");
            let path = BinaryInstaller::install(
                &bin.repo,
                &bin.asset_pattern,
                bin.checksums_asset.as_deref(),
                install_dir,
                bin.release_tag.as_deref(),
            )
            .await?;
            binary_path_shared = Some(path.display().to_string());
            ui::print_success(&format!("Binary installed → {}", path.display()));
            // Hint if the install dir is not in PATH
            let install_dir_path = path.parent().unwrap_or(std::path::Path::new(""));
            let in_path = std::env::var("PATH").unwrap_or_default()
                .split(':')
                .any(|p| std::path::Path::new(p) == install_dir_path);
            if !in_path {
                println!("  Hint: add {} to your PATH to use the binary directly:", install_dir_path.display());
                println!("    export PATH=\"{}:$PATH\"", install_dir_path.display());
            }
            components.push("binary".to_string());
        }

        // Install Python package via pip/pipx
        if let Some(ref py) = plugin.components.python {
            PythonInstaller::install(&py.install_command, &plugin.name)?;
            ui::print_success(&format!("Python package installed via: {}", py.install_command));
            components.push("python".to_string());
        }

        // Install Node.js/TypeScript package via npm
        if let Some(ref npm) = plugin.components.npm {
            NpmInstaller::install(&npm.install_command, &plugin.name)?;
            ui::print_success(&format!("npm package installed via: {}", npm.install_command));
            components.push("npm".to_string());
        }
    }

    for agent_kind in &target_agents {
        let adapter = get_adapter(agent_kind);
        let existing_agent = existing.as_ref()
            .and_then(|p| p.agents.iter().find(|a| a.agent == agent_kind.id()));
        let mut agent_record = InstalledAgent {
            agent: agent_kind.id().to_string(),
            // Preserve existing skill data when doing --mcp-only
            skill_path: if mcp_only { existing_agent.and_then(|a| a.skill_path.clone()) } else { None },
            skill_names: if mcp_only { existing_agent.map(|a| a.skill_names.clone()).unwrap_or_default() } else { Vec::new() },
            // Preserve existing mcp data when doing --skill-only
            mcp_key: if skill_only { existing_agent.and_then(|a| a.mcp_key.clone()) } else { None },
            mcp_keys: if skill_only { existing_agent.map(|a| a.mcp_keys.clone()).unwrap_or_default() } else { Vec::new() },
            binary_path: binary_path_shared.clone(),
        };

        // Install skill
        if !mcp_only {
            if let Some(ref skill) = plugin.components.skill {
                // Determine the git ref to use for downloads
                let git_ref = skill.commit.as_deref().unwrap_or("main");

                // Warn if community plugin has no pinned commit
                if plugin.source == "community" && skill.commit.is_none() {
                    eprintln!(
                        "  Warning: community plugin '{}' has no pinned commit. \
                         Content may change without notice.",
                        plugin.name
                    );
                }

                if let Some(ref dir) = skill.dir {
                    // Directory mode: install SKILL.md + all siblings (e.g. references/)
                    let skill_dir = adapter.skill_dir(&plugin.name);
                    let count = SkillInstaller::install_from_dir(&skill.repo, dir, &skill_dir, git_ref).await?;
                    agent_record.skill_path = Some(skill_dir.join("SKILL.md").display().to_string());
                    ui::print_success(&format!(
                        "Skill installed → {} ({}, {} files)",
                        skill_dir.display(),
                        agent_kind.name(),
                        count
                    ));
                } else if let Some(ref path) = skill.path {
                    // Legacy single-file mode
                    let content = SkillInstaller::download_from_github(&skill.repo, path, git_ref).await?;
                    let skill_dir = adapter.skill_dir(&plugin.name);
                    SkillInstaller::write_skill(&skill_dir, &content)?;
                    agent_record.skill_path = Some(skill_dir.join("SKILL.md").display().to_string());
                    ui::print_success(&format!(
                        "Skill installed → {} ({})",
                        skill_dir.display(),
                        agent_kind.name()
                    ));
                } else {
                    // Auto-discover mode: scan repo tree for SKILL.md + .mcp.json
                    // Guard: refuse to auto-discover from the plugin-store registry repo itself,
                    // as that would install every skill in the repo.
                    if skill.repo == plugin_store::config::registry_repo() {
                        anyhow::bail!(
                            "Plugin '{}' has no skill path or dir configured. \
                             Run `plugin-store registry update` to refresh the registry cache.",
                            plugin.name
                        );
                    }
                    let result = SkillInstaller::discover_all(&skill.repo, git_ref).await?;
                    if result.skills.is_empty() {
                        eprintln!("  Warning: no SKILL.md files found in {}", skill.repo);
                    }
                    let mut first_path: Option<String> = None;
                    for ds in &result.skills {
                        let skill_dir = adapter.skill_dir(&ds.name);
                        SkillInstaller::install_discovered_skill(
                            &skill.repo,
                            ds,
                            &skill_dir,
                            git_ref,
                        )
                        .await?;
                        agent_record.skill_names.push(ds.name.clone());
                        if first_path.is_none() {
                            first_path = Some(skill_dir.join("SKILL.md").display().to_string());
                        }
                        ui::print_success(&format!(
                            "Skill installed → {} ({}, {} files)",
                            skill_dir.display(),
                            ds.name,
                            ds.files.len()
                        ));
                    }
                    agent_record.skill_path = first_path;

                    // Auto-install discovered MCP servers (unless --skill-only)
                    if !skill_only && plugin.components.mcp.is_none() && !result.mcps.is_empty() {
                        for mcp in &result.mcps {
                            McpInstaller::install(
                                agent_kind,
                                &mcp.name,
                                &mcp.command,
                                &mcp.args,
                                &mcp.env,
                            )?;
                            agent_record.mcp_keys.push(mcp.name.clone());
                            if agent_record.mcp_key.is_none() {
                                agent_record.mcp_key = Some(mcp.name.clone());
                            }
                            ui::print_success(&format!(
                                "MCP discovered & configured: {} ({})",
                                mcp.name,
                                agent_kind.name()
                            ));
                        }
                        if !components.contains(&"mcp".to_string()) {
                            components.push("mcp".to_string());
                        }
                    }
                }
                if !components.contains(&"skill".to_string()) {
                    components.push("skill".to_string());
                }
            }
        }

        // Install MCP
        if !skill_only {
            if let Some(ref mcp) = plugin.components.mcp {
                let mcp_name = format!("{}-mcp", plugin.name);
                McpInstaller::install(agent_kind, &mcp_name, &mcp.command, &mcp.args, &mcp.env)?;
                agent_record.mcp_key = Some(mcp_name.clone());
                ui::print_success(&format!(
                    "MCP configured for {}",
                    agent_kind.name()
                ));
                if !components.contains(&"mcp".to_string()) {
                    components.push("mcp".to_string());
                }
            }
        }

        // If partial install produced no new mcp/skill entries, keep existing ones
        if mcp_only && agent_record.mcp_keys.is_empty() && agent_record.mcp_key.is_none() {
            if let Some(ea) = existing_agent {
                agent_record.mcp_keys = ea.mcp_keys.clone();
                agent_record.mcp_key = ea.mcp_key.clone();
            }
        }
        if skill_only && agent_record.skill_names.is_empty() && agent_record.skill_path.is_none() {
            if let Some(ea) = existing_agent {
                agent_record.skill_names = ea.skill_names.clone();
                agent_record.skill_path = ea.skill_path.clone();
            }
        }

        installed_agents.push(agent_record);
    }

    // Record state
    let record = InstalledPlugin {
        name: plugin.name.clone(),
        version: plugin.version.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        agents: installed_agents,
        components_installed: components,
    };
    state_mgr.add(record)?;

    println!("{}", "Done!".green().bold());

    // Fire-and-forget: report install to stats endpoint
    stats::report_install(&plugin.name, &plugin.version, stats_url.as_deref()).await;

    Ok(())
}
