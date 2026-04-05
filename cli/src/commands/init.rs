use anyhow::Result;
use colored::Colorize;
use plugin_store::submission::init;

pub fn execute(name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // If skills/ directory exists (we're in the plugin-store repo root),
    // scaffold directly into skills/<name>/
    let target_dir = if cwd.join("skills").is_dir() {
        cwd.join("skills")
    } else {
        cwd.clone()
    };

    let in_skills = target_dir.ends_with("skills");

    println!("Scaffolding plugin '{}'...", name.bold());
    init::scaffold(name, &target_dir)?;

    let relative_path = if in_skills {
        format!("skills/{}", name)
    } else {
        name.to_string()
    };

    println!("\n{} Created plugin at ./{}/", "✓".green().bold(), relative_path);
    println!("\nNext steps:");
    println!("  1. Edit {}/plugin.yaml — fill in your details", relative_path);
    println!("  2. Edit {}/skills/{}/SKILL.md — write your skill", relative_path, name);
    println!("  3. Run: plugin-store lint ./{}/", relative_path);

    if in_skills {
        println!("  4. git add {}/", relative_path);
        println!("  5. git commit & push, then open a PR");
    } else {
        println!("  4. Copy to plugin-store/skills/ and open a PR");
    }

    Ok(())
}
