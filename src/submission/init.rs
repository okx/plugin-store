use anyhow::{bail, Result};
use std::path::Path;

/// Scaffold a new plugin submission directory.
pub fn scaffold(name: &str, target_dir: &Path) -> Result<()> {
    let re = regex::Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").unwrap();
    if name.len() < 2 || name.len() > 40 || !re.is_match(name) {
        bail!(
            "Invalid plugin name '{}'. Must be 2-40 chars, lowercase alphanumeric with hyphens.",
            name
        );
    }

    let reserved = ["okx-", "official-", "plugin-store-"];
    for prefix in reserved {
        if name.starts_with(prefix) {
            bail!(
                "Name '{}' uses reserved prefix '{}'. Choose a different name.",
                name,
                prefix
            );
        }
    }

    let plugin_dir = target_dir.join(name);
    if plugin_dir.exists() {
        bail!("Directory '{}' already exists", plugin_dir.display());
    }

    let skills_dir = plugin_dir.join("skills").join(name);
    let refs_dir = skills_dir.join("references");
    std::fs::create_dir_all(&refs_dir)?;

    // ── plugin.yaml ───────────────────────────────────────────────
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            r#"# Plugin manifest — see CONTRIBUTING.md for full guide
schema_version: 1
name: {name}
# alias: "Display Name"
version: "1.0.0"
description: "TODO: One-line description of what this plugin does"
author:
  name: "TODO: Your Name"
  github: "TODO: your-github-username"
  # email: "you@example.com"
license: MIT
category: utility    # trading-strategy | defi-protocol | analytics | utility | security | wallet | nft
tags:
  - TODO

components:
  skill:
    dir: skills/{name}
  # binary:                          # Uncomment if providing a compiled binary
  #   repo: your-org/your-repo       # (requires build section)
  #   asset_pattern: "my-binary-{{target}}"
  #   checksums_asset: checksums.txt

# Chains this plugin operates on (for reviewer reference)
chains:
  - ethereum

# External API domains this plugin calls (for reviewer reference)
# Undeclared URLs in SKILL.md will trigger a lint warning
api_calls: []
# api_calls:
#   - "api.example.com"
"#
        ),
    )?;

    // ── SKILL.md ──────────────────────────────────────────────────
    std::fs::write(
        skills_dir.join("SKILL.md"),
        format!(
            r#"---
name: {name}
description: "TODO: Brief description of what this skill does"
version: "1.0.0"
author: "TODO: Your Name"
tags:
  - TODO
---

# {name}

## Overview

TODO: Describe what this skill enables the AI agent to do.

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured
2. TODO: List any other prerequisites

## Commands

### Command 1: TODO

```bash
onchainos token search --query "ETH"
```

**When to use**: TODO
**Output**: TODO

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| TODO  | TODO  | TODO       |

## Skill Routing

- For token swaps → use `okx-dex-swap` skill
- For security scanning → use `okx-security` skill
"#
        ),
    )?;

    // ── references/cli-reference.md ───────────────────────────────
    std::fs::write(
        refs_dir.join("cli-reference.md"),
        format!(
            "# {name} CLI Reference\n\n\
             ## Commands\n\n\
             TODO: Document the onchainos commands this skill uses.\n\n\
             ```bash\n\
             onchainos token search --query \"ETH\" --chain ethereum\n\
             ```\n"
        ),
    )?;

    // ── LICENSE ───────────────────────────────────────────────────
    let year = chrono::Utc::now().format("%Y");
    std::fs::write(
        plugin_dir.join("LICENSE"),
        format!(
            "MIT License\n\n\
             Copyright (c) {year} TODO: Your Name\n\n\
             Permission is hereby granted, free of charge, to any person obtaining a copy\n\
             of this software and associated documentation files (the \"Software\"), to deal\n\
             in the Software without restriction, including without limitation the rights\n\
             to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
             copies of the Software, and to permit persons to whom the Software is\n\
             furnished to do so, subject to the following conditions:\n\n\
             The above copyright notice and this permission notice shall be included in all\n\
             copies or substantial portions of the Software.\n\n\
             THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n\
             IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n\
             FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n\
             AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n\
             LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n\
             OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n\
             SOFTWARE.\n"
        ),
    )?;

    // ── CHANGELOG.md ─────────────────────────────────────────────
    std::fs::write(
        plugin_dir.join("CHANGELOG.md"),
        "# Changelog\n\n## 1.0.0\n\n- Initial release\n",
    )?;

    // ── README.md ────────────────────────────────────────────────
    std::fs::write(
        plugin_dir.join("README.md"),
        format!(
            "# {name}\n\n\
             TODO: Describe your plugin.\n\n\
             ## Installation\n\n\
             ```bash\n\
             plugin-store install {name}\n\
             ```\n\n\
             ## What it does\n\n\
             TODO: Explain what this plugin enables.\n\n\
             ## License\n\n\
             MIT\n"
        ),
    )?;

    Ok(())
}
