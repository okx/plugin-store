use colored::Colorize;
use dialoguer::{Confirm, MultiSelect};
use crate::agent::DetectedAgent;

pub fn print_success(msg: &str) {
    println!("  {} {}", "✔".green(), msg);
}

pub fn print_error(msg: &str) {
    eprintln!("  {} {}", "✗".red(), msg);
}

pub fn print_warning(msg: &str) {
    eprintln!("  {} {}", "⚠".yellow(), msg);
}

pub fn confirm_community_plugin(plugin_name: &str) -> bool {
    Confirm::new()
        .with_prompt(format!(
            "⚠ '{}' is a community plugin, not officially verified. Continue?",
            plugin_name
        ))
        .default(false)
        .interact()
        .unwrap_or(false)
}

pub fn select_agents(agents: &[DetectedAgent]) -> Vec<usize> {
    println!("\nDetected agents:");
    let items: Vec<String> = agents
        .iter()
        .map(|a| {
            let status = if a.found {
                "✔".green().to_string()
            } else {
                "✗".red().to_string()
            };
            format!("{} {} ({})", status, a.kind.name(), a.path_hint)
        })
        .collect();

    let available: Vec<usize> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.found)
        .map(|(i, _)| i)
        .collect();

    if available.is_empty() {
        print_error("No supported agents detected.");
        return vec![];
    }

    let defaults: Vec<bool> = agents.iter().map(|a| a.found).collect();

    MultiSelect::new()
        .with_prompt("Select target agents")
        .items(&items)
        .defaults(&defaults)
        .interact()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| agents[*i].found)
        .collect()
}
