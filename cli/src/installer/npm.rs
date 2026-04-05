use anyhow::{bail, Context, Result};

pub struct NpmInstaller;

impl NpmInstaller {
    /// Install a Node.js/TypeScript package via npm from a git repo.
    pub fn install(install_command: &str, package_name: &str) -> Result<String> {
        let git_url = install_command
            .strip_prefix("npm install -g ")
            .unwrap_or(install_command);

        let npm = find_npm().ok_or_else(|| {
            anyhow::anyhow!(
                "npm not found. Install Node.js first.\n\
                 Then run manually: npm install -g {}",
                git_url
            )
        })?;

        println!("  Running: {} install -g {}", npm, git_url);

        let status = std::process::Command::new(npm)
            .args(["install", "-g", git_url])
            .status()
            .context(format!("Failed to run {}", npm))?;

        if !status.success() {
            bail!(
                "{} exited with code {}. You can retry manually:\n  npm install -g {}",
                npm,
                status.code().unwrap_or(-1),
                git_url
            );
        }

        if command_exists(package_name) {
            println!("  Verified: `{}` is now available on PATH", package_name);
        } else {
            println!(
                "  Note: `{}` may not be on your PATH yet. Check `npm bin -g` for the install location.",
                package_name
            );
        }

        Ok(install_command.to_string())
    }

    /// Uninstall a Node.js package.
    pub fn uninstall(package_name: &str) -> Result<()> {
        let npm = match find_npm() {
            Some(n) => n,
            None => return Ok(()),
        };

        let status = std::process::Command::new(npm)
            .args(["uninstall", "-g", package_name])
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                eprintln!("  Warning: could not uninstall npm package '{}'", package_name);
                Ok(())
            }
        }
    }
}

fn find_npm() -> Option<&'static str> {
    if command_exists("npm") {
        Some("npm")
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
