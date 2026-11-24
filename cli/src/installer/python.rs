use anyhow::{bail, Context, Result};

pub struct PythonInstaller;

impl PythonInstaller {
    /// Install a Python package via pipx/pip/pip3 using the install_command from registry.
    /// Returns the install command that was executed (for state tracking).
    pub fn install(install_command: &str, package_name: &str) -> Result<String> {
        let git_url = install_command
            .strip_prefix("pip install ")
            .unwrap_or(install_command);

        // Prefer pipx (isolated env) > pip3 > pip
        let (cmd, args) = if command_exists("pipx") {
            ("pipx", vec!["install", git_url])
        } else if let Some(pip) = find_pip() {
            (pip, vec!["install", git_url])
        } else {
            bail!(
                "Neither pipx nor pip/pip3 found. Install Python 3 first.\n\
                 Then run manually: {}",
                install_command
            );
        };

        println!("  Running: {} {}", cmd, args.join(" "));

        let status = std::process::Command::new(cmd)
            .args(&args)
            .status()
            .context(format!("Failed to run {}", cmd))?;

        if !status.success() {
            bail!(
                "{} exited with code {}. You can retry manually:\n  {}",
                cmd,
                status.code().unwrap_or(-1),
                install_command
            );
        }

        // Verify the CLI binary is now available
        if command_exists(package_name) {
            println!("  Verified: `{}` is now available on PATH", package_name);
        } else {
            println!(
                "  Note: `{}` may not be on your PATH yet. If using pipx, run `pipx ensurepath`.",
                package_name
            );
        }

        Ok(install_command.to_string())
    }

    /// Uninstall a Python package.
    pub fn uninstall(package_name: &str) -> Result<()> {
        let (cmd, args) = if command_exists("pipx") {
            ("pipx", vec!["uninstall", package_name])
        } else if let Some(pip) = find_pip() {
            (pip, vec!["uninstall", "-y", package_name])
        } else {
            return Ok(());
        };

        let status = std::process::Command::new(cmd)
            .args(&args)
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                eprintln!("  Warning: could not uninstall Python package '{}'", package_name);
                Ok(())
            }
        }
    }
}

/// Find pip or pip3, whichever is available.
fn find_pip() -> Option<&'static str> {
    if command_exists("pip3") {
        Some("pip3")
    } else if command_exists("pip") {
        Some("pip")
    } else {
        None
    }
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
