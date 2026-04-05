use anyhow::Result;
use colored::Colorize;
use plugin_store::submission::lint;

pub fn execute(path: &str) -> Result<()> {
    let dir = std::path::Path::new(path);
    if !dir.exists() {
        anyhow::bail!("Path '{}' does not exist", path);
    }
    if !dir.is_dir() {
        anyhow::bail!("'{}' is not a directory", path);
    }

    println!("Linting {}...\n", dir.display());

    let report = lint::lint_submission(dir)?;

    for diag in &report.diagnostics {
        println!("  {}", diag);
    }

    println!();

    let errors = report.error_count();
    let warnings = report.warning_count();

    if errors == 0 && warnings == 0 {
        println!(
            "{} Plugin '{}' passed all checks!",
            "✓".green().bold(),
            report.plugin_name.bold()
        );
    } else if errors == 0 {
        println!(
            "{} Plugin '{}' passed with {} warning(s)",
            "✓".yellow().bold(),
            report.plugin_name.bold(),
            warnings
        );
    } else {
        println!(
            "{} Plugin '{}': {} error(s), {} warning(s)",
            "✗".red().bold(),
            report.plugin_name.bold(),
            errors,
            warnings
        );
        println!(
            "\nFix all errors before submitting. See CONTRIBUTING.md for guidance."
        );
        std::process::exit(1);
    }

    Ok(())
}
